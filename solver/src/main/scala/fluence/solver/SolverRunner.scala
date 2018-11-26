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

package fluence.solver
import java.io.{File, FileNotFoundException}

import cats.Monad
import cats.data.EitherT
import cats.effect.{ExitCode, IO, IOApp}
import fluence.solver.vm.VmOperationInvoker
import fluence.statemachine.StateMachine
import fluence.vm.VmError.WasmVmError
import fluence.vm.{VmError, WasmVm}
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LazyLogging, LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.util.Try

/**
 * Solver settings.
 *
 * @param logLevel level of logging ( OFF / ERROR / WARN / INFO / DEBUG / TRACE )
 * @param moduleFiles sequence of files with WASM module code
 *
 **/
case class SolverConfig(logLevel: String, moduleFiles: List[String])

/**
 * Main class for the State machine.
 *
 * Launches JVM ABCI Server by instantiating jTendermint's `TSocket` registering `ABCIHandler` as handler.
 * jTendermint implements RPC layer, provides dedicated threads (`Consensus`, `Mempool` and `Query` thread
 * according to Tendermint specification) and sends ABCI requests to `ABCIHandler`.
 */
object SolverRunner extends IOApp with LazyLogging {
  val DefaultABCIPort: Int = 26658 // default Tendermint ABCI port
  val DefaultMetricsPort: Int = 26661 // default Prometheus metrics port

  override def run(args: List[String]): IO[ExitCode] = {
    val abciPort = if (args.nonEmpty) args.head.toInt else DefaultABCIPort
    val metricsPort = if (args.length > 1) args(1).toInt else DefaultMetricsPort
    SolverRunner
      .start(abciPort, metricsPort)
      .map(_ => ExitCode.Success)
      .valueOr(error => {
        logger.error("Error during State machine run: " + error + " caused by: " + error.causedBy)
        ExitCode.Error
      })
  }

  /**
   * Starts the State machine.
   *
   * @param abciPort port used to listen to Tendermint requests
   * @param metricsPort port used to provide Prometheus metrics
   */
  private def start(abciPort: Int, metricsPort: Int): EitherT[IO, SolverError, Unit] =
    for {
      _ <- configureLogging()
      config <- loadConfig()
//      _ = configureLogLevel(config.logLevel)

//      _ = logger.info("Starting Metrics servlet")
//      _ = startMetricsServer(metricsPort)

      moduleFilenames <- moduleFilesFromConfig[IO](config)
      _ = logger.info("Loading VM modules from " + moduleFilenames)
      vm <- buildVm[IO](moduleFilenames).leftMap(e => VMInitializationError(e.toString))
      invoker = new VmOperationInvoker[IO](vm)
      _ <- StateMachine
        .listen(invoker, abciPort)
        .leftMap(e => StateMachineInitializationError(e.toString): SolverError)
    } yield ()

  /**
   * Builds a VM instance used to perform function calls from the clients.
   *
   * @param moduleFiles module filenames with VM code
   */
  private def buildVm[F[_]: Monad](moduleFiles: Seq[String]): EitherT[F, WasmVmError.ApplyError, WasmVm] =
    WasmVm[F](moduleFiles) //.leftMap(Invoker.convertToStateMachineError)

  /**
   * Extracts module filenames from config with particular files and directories with files mixed.
   *
   * @param config config object to load VM setting
   * @return either a sequence of filenames found in directories and among files provided in config
   *         or error denoting a specific problem with locating one of directories and files from config
   */
  private def moduleFilesFromConfig[F[_]: Monad](
    config: SolverConfig
  ): EitherT[F, SolverError, Seq[String]] =
    EitherT.fromEither[F](
      config.moduleFiles
        .map(
          name =>
            Try({
              val file = new File(name)
              if (!file.exists())
                Left(new FileNotFoundException(name))
              else if (file.isDirectory)
                Right(file.listFiles().toList)
              else
                Right(List(file))
            }).toEither
              .flatMap(identity)
              .left
              .map(x => VmModuleLocationError("Error during locating VM module files and directories", x))
        )
        .partition(_.isLeft) match {
        case (Nil, files) => Right(for (Right(f) <- files) yield f).map(_.flatten.map(_.getPath))
        case (errors, _) => Left(for (Left(s) <- errors) yield s).left.map(_.head)
      }
    )

  /**
   * Configures `slogging` logger.
   */
  private def configureLogging(): EitherT[IO, Nothing, Unit] =
    EitherT.right(IO {
      PrintLoggerFactory.formatter = new DefaultPrefixFormatter(false, false, false)
      LoggerConfig.factory = PrintLoggerFactory()
      LoggerConfig.level = LogLevel.INFO
    })

  /**
   * Loads State machine config using `pureconfig` Scala config loading mechanism.
   */
  private def loadConfig(): EitherT[IO, SolverError, SolverConfig] =
    EitherT
      .fromEither[IO](pureconfig.loadConfig[SolverConfig])
      .leftMap(
        e => ConfigLoadingError("Unable to read solver config: " + e.toList)
      )
  /**
   * Starts metrics servlet on provided port
   *
   * @param metricsPort port to expose Prometheus metrics
   */
  //  private def startMetricsServer(metricsPort: Int): Unit = {
  //    val server = new Server(metricsPort)
  //    val context = new ServletContextHandler
  //    context.setContextPath("/")
  //    server.setHandler(context)
  //
  //    context.addServlet(new ServletHolder(new MetricsServlet()), "/")
  //    server.start()
  //  }
}

/**
 * Base trait for errors occurred in Solver.
 *
 * @param code short text code describing error, might be shown to the client
 * @param message detailed error message
 * @param causedBy caught [[Throwable]], if any
 */
sealed abstract class SolverError(val code: String, val message: String, val causedBy: Option[Throwable])

/**
 * Corresponds to errors occurred during looking for VM module files before passing them to VM.
 *
 * @param message detailed error message
 * @param throwable caught [[Throwable]]
 */
case class VmModuleLocationError(override val message: String, throwable: Throwable)
    extends SolverError("VmModuleLocationError", message, Some(throwable))

///**
// * Corresponds to errors occurred during VM function invocation inside VM.
// *
// * @param code short text code describing error, might be shown to the client
// * @param message detailed error message
// * @param vmError caught [[VmError]]
// */
//case class VmRuntimeError(override val code: String, override val message: String, vmError: VmError)
//    extends SolverError(code, message, Some(vmError))

case class ConfigLoadingError(override val message: String) extends SolverError("ConfigLoadingError", message, None)

case class VMInitializationError(override val message: String)
    extends SolverError("VmInitializationError", message, None)

case class StateMachineInitializationError(override val message: String)
    extends SolverError("StateMachineInitializationError", message, None)
