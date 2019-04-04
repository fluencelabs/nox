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
import com.github.jtendermint.jabci.socket.TSocket
import com.softwaremill.sttp.SttpBackend
import fluence.EitherTSttpBackend
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.statemachine.config.StateMachineConfig
import fluence.statemachine.control.{ControlServer, ControlSignals}
import fluence.statemachine.error.StateMachineError
import fluence.vm.WasmVm
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging._

import scala.language.higherKinds

/**
 * Main class for the State machine.
 *
 * Launches JVM ABCI Server by instantiating jTendermint's `TSocket` registering `ABCIHandler` as handler.
 * jTendermint implements RPC layer, provides dedicated threads (`Consensus`, `Mempool` and `Query` thread
 * according to Tendermint specification) and sends ABCI requests to `ABCIHandler`.
 */
object ServerRunner extends IOApp with LazyLogging {

  private val sttpResource: Resource[IO, SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]]] =
    Resource.make(IO(EitherTSttpBackend[IO]()))(sttpBackend ⇒ IO(sttpBackend.close()))

  override def run(args: List[String]): IO[ExitCode] = {
    for {
      config <- StateMachineConfig.load[IO]()
      _ = configureLogging(convertLogLevel(config.logLevel))

      _ = logger.info("Building State Machine ABCI handler")
      _ <- (
        for {
          control ← ControlServer.make[IO](config.control)

          sttp ← sttpResource

          rpc ← {
            implicit val s = sttp
            TendermintRpc.make[IO](config.tendermintRpc.host, config.tendermintRpc.port)
          }

          _ ← abciHandlerResource(config.abciPort, config, control, rpc)
        } yield control.signals.stop
      ).use(identity)
    } yield ExitCode.Success
  }.guaranteeCase {
    case Canceled =>
      IO(logger.error("StateMachine was canceled"))
    case Error(e) =>
      IO(logger.error("StateMachine stopped with error: {}", e)).map(_ => e.printStackTrace(System.err))
    case Completed =>
      IO(logger.info("StateMachine exited gracefully"))
  }

  private def abciHandlerResource(
    abciPort: Int,
    config: StateMachineConfig,
    controlServer: ControlServer[IO],
    rpc: TendermintRpc[IO]
  ): Resource[IO, Unit] =
    Resource
      .make(
        buildAbciHandler(config, controlServer.signals, rpc).value.flatMap {
          case Right(handler) ⇒ IO.pure(handler)
          case Left(err) ⇒ IO.raiseError(new RuntimeException("Building ABCI handler failed: " + err))
        }.flatMap { handler ⇒
          IO {
            logger.info("Starting State Machine ABCI handler")
            val socket = new TSocket
            socket.registerListener(handler)

            val socketThread = new Thread(() => socket.start(abciPort))
            socketThread.setName("AbciSocket")
            socketThread.start()
            socketThread
          }
        }
      )(socketThread ⇒ IO(if (socketThread.isAlive) socketThread.interrupt()))
      .map(_ ⇒ logger.info("State Machine ABCI handler started successfully"))

  /**
   * Builds [[AbciHandler]], used to serve all Tendermint requests.
   *
   * @param config config object to load various settings
   */
  private[statemachine] def buildAbciHandler(
    config: StateMachineConfig,
    controlSignals: ControlSignals[IO],
    rpc: TendermintRpc[IO]
  ): EitherT[IO, StateMachineError, AbciHandler[IO]] =
    for {
      moduleFilenames <- config.collectModuleFiles[IO]
      _ = logger.info("Loading VM modules from " + moduleFilenames)
      vm <- buildVm[IO](moduleFilenames)

      _ = logger.info("VM instantiated")

      vmInvoker = new VmOperationInvoker[IO](vm)

      service <- EitherT.right(AbciService[IO](vmInvoker))
    } yield new AbciHandler[IO](service, controlSignals, rpc)

  /**
   * Builds a VM instance used to perform function calls from the clients.
   *
   * @param moduleFiles module filenames with VM code
   */
  private def buildVm[F[_]: Monad](moduleFiles: NonEmptyList[String]): EitherT[F, StateMachineError, WasmVm] =
    WasmVm[F](moduleFiles).leftMap(VmOperationInvoker.convertToStateMachineError)

  /**
   * Configures `slogging` logger.
   */
  private def configureLogging(level: LogLevel): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(true, true, true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = level
  }

  /**
   * Converts String to `slogging` log level.
   *
   * @param logLevel level of logging
   */
  private def convertLogLevel(logLevel: String): LogLevel =
    logLevel.toUpperCase match {
      case "OFF" => LogLevel.OFF
      case "ERROR" => LogLevel.ERROR
      case "WARN" => LogLevel.WARN
      case "INFO" => LogLevel.INFO
      case "DEBUG" => LogLevel.DEBUG
      case "TRACE" => LogLevel.TRACE
      case _ =>
        logger.warn(s"Unknown log level `$logLevel`; falling back to INFO")
        LogLevel.INFO
    }

}
