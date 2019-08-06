/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.statemachine

import java.nio.ByteBuffer

import cats.Monad
import cats.data.{EitherT, NonEmptyList}
import cats.effect.ExitCase.{Canceled, Completed, Error}
import cats.effect.{ExitCode, IO, IOApp, Resource}
import cats.syntax.apply._
import cats.syntax.flatMap._
import com.github.jtendermint.jabci.socket.TSocket
import com.softwaremill.sttp.SttpBackend
import fluence.EitherTSttpBackend
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.log.{Log, LogFactory, LogLevel}
import fluence.statemachine.config.StateMachineConfig
import fluence.statemachine.control.{ControlServer, ControlSignals}
import fluence.statemachine.error.StateMachineError
import fluence.statemachine.vm.WasmVmOperationInvoker
import fluence.vm.WasmVm
import fluence.vm.wasm.MemoryHasher
import LogLevel.toLogLevel

import scala.language.higherKinds

/**
 * Main class for the State machine.
 *
 * Launches JVM ABCI Server by instantiating jTendermint's `TSocket` registering `ABCIHandler` as handler.
 * jTendermint implements RPC layer, provides dedicated threads (`Consensus`, `Mempool` and `Query` thread
 * according to Tendermint specification) and sends ABCI requests to `ABCIHandler`.
 */
object ServerRunner extends IOApp {

  private val sttpResource: Resource[IO, SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]]] =
    Resource.make(IO(EitherTSttpBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  override def run(args: List[String]): IO[ExitCode] =
    StateMachineConfig
      .load[IO]()
      .flatMap { config =>
        val logLevel = LogLevel.fromString(config.logLevel).getOrElse(LogLevel.INFO)
        implicit val logFactory: LogFactory[IO] =
          LogFactory.forPrintln[IO](logLevel)
        for {
          implicit0(log: Log[IO]) ← logFactory.init("server")
          _ ← log.info("Building State Machine ABCI handler")
          _ <- (
            for {
              control ← ControlServer.make[IO](config.control)

              sttp ← sttpResource

              tendermintRpc ← {
                implicit val s = sttp
                TendermintRpc.make[IO](config.tendermintRpc.host, config.tendermintRpc.port)
              }

              _ ← abciHandlerResource(config.abciPort, config, control, tendermintRpc)
            } yield control.signals.stop
          ).use(identity)
        } yield ExitCode.Success
      }
      .guaranteeCase {
        case Canceled =>
          LogFactory.forPrintln[IO]().init("server", "shutdown") >>= (_.error("StateMachine was canceled"))
        case Error(e) =>
          LogFactory.forPrintln[IO]().init("server", "shutdown") >>= (_.error("StateMachine stopped with error", e))
        case Completed =>
          LogFactory.forPrintln[IO]().init("server", "shutdown") >>= (_.info("StateMachine exited gracefully"))
      }

  private def abciHandlerResource(
    abciPort: Int,
    config: StateMachineConfig,
    controlServer: ControlServer[IO],
    tendermintRpc: TendermintRpc[IO]
  )(implicit log: Log[IO], lf: LogFactory[IO]): Resource[IO, Unit] =
    Resource
      .make(
        buildAbciHandler(config, controlServer.signals).value.flatMap {
          case Right(handler) ⇒ IO.pure(handler)
          case Left(err) ⇒
            val exception = err.causedBy match {
              case Some(caused) => new RuntimeException("Building ABCI handler failed: " + err, caused)
              case None         => new RuntimeException("Building ABCI handler failed: " + err)
            }
            IO.raiseError(exception)
        }.flatMap { handler ⇒
          Log[IO].info("Starting State Machine ABCI handler") >>
            IO {
              val socket = new TSocket
              socket.registerListener(handler)

              val socketThread = new Thread(() => socket.start(abciPort))
              socketThread.setName("AbciSocket")
              socketThread.start()

              (socketThread, socket)
            }
        }
      ) {
        case (socketThread, socket) ⇒
          log.info(s"Stopping TSocket and its thread") *>
            IO {
              socket.stop()
              if (socketThread.isAlive) socketThread.interrupt()
            }
      }
      .flatMap(_ ⇒ Log.resource[IO].info("State Machine ABCI handler started successfully"))

  /**
   * Builds [[AbciHandler]], used to serve all Tendermint requests.
   *
   * @param config config object to load various settings
   */
  private[statemachine] def buildAbciHandler(
    config: StateMachineConfig,
    controlSignals: ControlSignals[IO],
  )(implicit log: Log[IO], lf: LogFactory[IO]): EitherT[IO, StateMachineError, AbciHandler[IO]] =
    for {
      moduleFilenames <- config.collectModuleFiles[IO]
      _ ← Log.eitherT[IO, StateMachineError].info("Loading VM modules from " + moduleFilenames)
      vm <- buildVm[IO](moduleFilenames)

      _ ← Log.eitherT[IO, StateMachineError].info("VM instantiated")

      vmInvoker = new WasmVmOperationInvoker[IO](vm)

      service <- EitherT.right(AbciService[IO](vmInvoker, controlSignals, config.blockUploadingEnabled))
    } yield new AbciHandler[IO](service, controlSignals)

  /**
   * Builds a VM instance used to perform function calls from the clients.
   *
   * @param moduleFiles module filenames with VM code
   */
  private def buildVm[F[_]: Monad: Log](moduleFiles: NonEmptyList[String]): EitherT[F, StateMachineError, WasmVm] =
    WasmVm[F](moduleFiles, MemoryHasher[F]).leftMap(WasmVmOperationInvoker.convertToStateMachineError)
}
