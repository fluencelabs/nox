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

import java.io.{File, FileNotFoundException}

import cats.Monad
import cats.data.EitherT
import cats.effect.concurrent.MVar
import cats.effect.{ExitCode, IO, IOApp}
import com.github.jtendermint.jabci.socket.TSocket
import fluence.statemachine.config.StateMachineConfig
import fluence.statemachine.contract.ClientRegistry
import fluence.statemachine.error.{ConfigLoadingError, StateMachineError, VmModuleLocationError}
import fluence.statemachine.state._
import fluence.statemachine.tx.{TxParser, TxProcessor, TxStateDependentChecker, VmOperationInvoker}
import fluence.vm.WasmVm
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging._

import scala.language.higherKinds
import scala.util.Try

/**
 * Main class for the State machine.
 *
 * Launches JVM ABCI Server by instantiating jTendermint's `TSocket` registering `ABCIHandler` as handler.
 * jTendermint implements RPC layer, provides dedicated threads (`Consensus`, `Mempool` and `Query` thread
 * according to Tendermint specification) and sends ABCI requests to `ABCIHandler`.
 */
object ServerRunner extends IOApp with LazyLogging {
  val DefaultABCIPoint: Int = 26658

  override def run(args: List[String]): IO[ExitCode] = {
    val port = if (args.length > 0) args(0).toInt else DefaultABCIPoint
    ServerRunner
      .start(port)
      .map(_ => ExitCode.Success)
      .valueOr(error => {
        logger.error("Error during State machine run: " + error + " caused by: " + error.causedBy)
        ExitCode.Error
      })
  }

  /**
   * Starts the State machine.
   *
   * @param port port used to listen to Tendermint requests
   */
  private def start(port: Int): EitherT[IO, StateMachineError, Unit] =
    for {
      _ <- EitherT.right(IO { configureLogging() })

      _ = logger.info("Building State Machine ABCI handler")
      abciHandler <- buildAbciHandler()

      _ = logger.info("Starting State Machine ABCI handler")
      socket = new TSocket
      _ = socket.registerListener(abciHandler)

      socketThread = new Thread(() => socket.start(port))
      _ = socketThread.setName("Socket")
      _ = socketThread.start()
      _ = socketThread.join()
    } yield ()

  /**
   * Builds [[AbciHandler]], used to serve all Tendermint requests.
   */
  private[statemachine] def buildAbciHandler(): EitherT[IO, StateMachineError, AbciHandler] =
    for {
      config <- EitherT
        .fromEither[IO](pureconfig.loadConfig[StateMachineConfig])
        .leftMap(
          e => ConfigLoadingError("Unable to read StateMachineConfig: " + e.toList)
        )

      moduleFilenames <- moduleFilesFromConfig[IO](config)
      _ = logger.info("Loading VM modules from " + moduleFilenames)
      vm <- buildVm[IO](moduleFilenames)
      vmInvoker = new VmOperationInvoker[IO](vm)

      initialState <- EitherT.right(MVar[IO].of(TendermintState.initial))
      stateHolder = new TendermintStateHolder[IO](initialState)
      mutableConsensusState = new MutableStateTree(stateHolder)

      queryProcessor = new QueryProcessor(stateHolder)

      txParser = new TxParser[IO](new ClientRegistry())
      checkTxStateChecker = new TxStateDependentChecker[IO](stateHolder.mempoolState)
      deliverTxStateChecker = new TxStateDependentChecker(mutableConsensusState.getRoot)

      txProcessor = new TxProcessor(mutableConsensusState, vmInvoker, config)

      committer = new Committer[IO](stateHolder, vmInvoker)

      abciHandler = new AbciHandler(
        committer,
        queryProcessor,
        txParser,
        checkTxStateChecker,
        deliverTxStateChecker,
        txProcessor
      )
    } yield abciHandler

  /**
   * Builds a VM instance used to perform function calls from the clients.
   *
   * @param moduleFiles module filenames with VM code
   */
  private def buildVm[F[_]: Monad](moduleFiles: Seq[String]): EitherT[F, StateMachineError, WasmVm] =
    WasmVm[F](moduleFiles).leftMap(VmOperationInvoker.convertToStateMachineError)

  /**
   * Extracts module filenames from config with particular files and directories with files mixed.
   *
   * @param config config object to load VM setting
   * @return either a sequence of filenames found in directories and among files provided in config
   *         or error denoting a specific problem with locating one of directories and files from config
   */
  private def moduleFilesFromConfig[F[_]: Monad](
    config: StateMachineConfig
  ): EitherT[F, StateMachineError, Seq[String]] =
    EitherT.fromEither[F](
      config.moduleFiles
        .map(
          name =>
            Try({
              val file = new File(name)
              if (!file.exists())
                throw new FileNotFoundException(name)
              else if (file.isDirectory)
                file.listFiles().toList
              else
                List(file)
            }).toEither.left.map(x => VmModuleLocationError("Error during locating VM module files and directories", x))
        )
        .partition(_.isLeft) match {
        case (Nil, files) => Right(for (Right(f) <- files) yield f).map(_.flatten.map(_.getPath))
        case (errors, _) => Left(for (Left(s) <- errors) yield s).left.map(_.head)
      }
    )

  /**
   * Configures `slogging` logger.
   */
  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(true, false, true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO
  }
}
