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

import cats.Monad
import cats.data.{EitherT, NonEmptyList}
import cats.effect.concurrent.MVar
import cats.effect.{ExitCode, IO, IOApp, Resource}
import com.github.jtendermint.jabci.socket.TSocket
import com.github.jtendermint.jabci.types.Request.ValueCase.{CHECK_TX, DELIVER_TX}
import fluence.statemachine.config.StateMachineConfig
import fluence.statemachine.contract.ClientRegistry
import fluence.statemachine.control.{ControlServer, ControlSignals}
import fluence.statemachine.error.StateMachineError
import fluence.statemachine.state._
import fluence.statemachine.tx.{TxParser, TxProcessor, TxStateDependentChecker, VmOperationInvoker}
import fluence.statemachine.util.Metrics
import fluence.vm.WasmVm
import io.prometheus.client.exporter.MetricsServlet
import org.eclipse.jetty.server.Server
import org.eclipse.jetty.servlet.{ServletContextHandler, ServletHolder}
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
  val DefaultABCIPort: Int = 26658 // default Tendermint ABCI port
  val DefaultMetricsPort: Int = 26661 // default Prometheus metrics port

  override def run(args: List[String]): IO[ExitCode] = {
    val abciPort = if (args.nonEmpty) args.head.toInt else DefaultABCIPort
    val metricsPort = if (args.length > 1) args(1).toInt else DefaultMetricsPort

    configureLogging()

    for {
      config <- StateMachineConfig.load[IO]()
      _ = configureLogLevel(config.logLevel)

      _ = logger.info("Starting Metrics servlet")
      _ = startMetricsServer(metricsPort)

      _ = logger.info("Building State Machine ABCI handler")
      _ <- (
        for {
          control ← ControlServer.make[IO](config.controlConfig)
          _ ← abciHandlerResource(abciPort, config, control)
        } yield control.signals.stop
      ).use(identity)
    } yield ExitCode.Success
  }

  private def abciHandlerResource(
    abciPort: Int,
    config: StateMachineConfig,
    controlServer: ControlServer[IO]
  ): Resource[IO, Unit] =
    Resource
      .make(
        buildAbciHandler(config, controlServer.signals).value.flatMap {
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
      .map(_ ⇒ ())

  /**
   * Starts metrics servlet on provided port
   *
   * @param metricsPort port to expose Prometheus metrics
   */
  private def startMetricsServer(metricsPort: Int): Resource[IO, Unit] =
    Resource
      .make(IO {
        val server = new Server(metricsPort)
        val context = new ServletContextHandler
        context.setContextPath("/")
        server.setHandler(context)

        context.addServlet(new ServletHolder(new MetricsServlet()), "/")
        server.start()

        server
      })(server ⇒ IO(server.stop()))
      .map(_ ⇒ ())

  /**
   * Builds [[AbciHandler]], used to serve all Tendermint requests.
   *
   * @param config config object to load various settings
   */
  private[statemachine] def buildAbciHandler(
    config: StateMachineConfig,
    controlSignals: ControlSignals[IO]
  ): EitherT[IO, StateMachineError, AbciHandler] =
    for {
      moduleFilenames <- config.collectModuleFiles[IO]
      _ = logger.info("Loading VM modules from " + moduleFilenames)
      vm <- buildVm[IO](moduleFilenames)

      _ = Metrics.resetCollectors()

      vmInvoker = new VmOperationInvoker[IO](vm)

      initialState <- EitherT.right(MVar[IO].of(TendermintState.initial))
      stateHolder = new TendermintStateHolder[IO](initialState)
      mutableConsensusState = new MutableStateTree(stateHolder)

      queryProcessor = new QueryProcessor(stateHolder)

      txParser = new TxParser[IO](new ClientRegistry())
      checkTxStateChecker = new TxStateDependentChecker[IO](CHECK_TX, stateHolder.mempoolState)
      deliverTxStateChecker = new TxStateDependentChecker(DELIVER_TX, mutableConsensusState.getRoot)

      txProcessor = new TxProcessor(mutableConsensusState, vmInvoker, config)

      committer = new Committer[IO](stateHolder, vmInvoker)

      abciHandler = new AbciHandler(
        committer,
        queryProcessor,
        txParser,
        checkTxStateChecker,
        deliverTxStateChecker,
        txProcessor,
        controlSignals
      )
    } yield abciHandler

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
  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO
  }

  /**
   * Configures `slogging` log level.
   *
   * @param logLevel level of logging
   */
  private def configureLogLevel(logLevel: String): Unit =
    LoggerConfig.level = logLevel.toUpperCase match {
      case "OFF" => LogLevel.OFF
      case "ERROR" => LogLevel.ERROR
      case "WARN" => LogLevel.WARN
      case "INFO" => LogLevel.INFO
      case "DEBUG" => LogLevel.DEBUG
      case "TRACE" => LogLevel.TRACE
      case _ => LogLevel.INFO
    }

}
