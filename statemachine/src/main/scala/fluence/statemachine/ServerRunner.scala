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

import cats.effect.concurrent.MVar
import cats.effect.{ExitCode, IO, IOApp}
import com.github.jtendermint.jabci.socket.TSocket
import fluence.statemachine.contract.ClientRegistry
import fluence.statemachine.state._
import fluence.statemachine.tree.TreeNode
import fluence.statemachine.tx.{TxStateDependentChecker, TxParser, TxProcessor, VmOperationInvoker}
import fluence.vm.WasmVm
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging._

/**
 * Main class for the State machine.
 *
 * Launches JVM ABCI Server by instantiating jTendermint's `TSocket` registering `ABCIHandler` as handler.
 * jTendermint implements RPC layer, provides dedicated threads (`Consensus`, `Mempool` and `Query` thread
 * according to Tendermint specification) and sends ABCI requests to `ABCIHandler`.
 */
object ServerRunner extends IOApp with LazyLogging {
  val DefaultABCIPoint: Int = 46658

  override def run(args: List[String]): IO[ExitCode] = {
    val port = if (args.length > 0) args(0).toInt else DefaultABCIPoint
    ServerRunner.start(port).map(_ => ExitCode.Success)
  }

  /**
   * Starts the State machine.
   *
   * @param port port used to listen to Tendermint requests
   */
  private def start(port: Int): IO[Unit] =
    for {
      abciHandler <- buildAbciHandler()

      _ = configureLogging()

      _ = logger.info("starting State Machine")
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
  private[statemachine] def buildAbciHandler(): IO[AbciHandler] =
    for {
      initialState <- MVar[IO].of(TendermintState.initial)
      stateHolder = new TendermintStateHolder[IO](initialState)
      mutableConsensusState = new MutableStateTree(stateHolder)

      vm <- buildVm()
      vmInvoker = new VmOperationInvoker[IO](vm)

      queryProcessor = new QueryProcessor(stateHolder)

      txParser = new TxParser[IO](new ClientRegistry())
      checkTxStateChecker = new TxStateDependentChecker[IO](stateHolder.mempoolState)
      deliverTxStateChecker = new TxStateDependentChecker(mutableConsensusState.getRoot)
      txProcessor = new TxProcessor(mutableConsensusState, vmInvoker)

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
   */
  private def buildVm(): IO[WasmVm] = {
    val moduleFiles = List(
      "vm/src/test/resources/wast/mul.wast",
      "vm/src/test/resources/wast/counter.wast"
    )
    WasmVm[IO](moduleFiles).value.map(
      ei => ei.getOrElse { throw new RuntimeException("Error loading VM: " + ei.left.get.toString) }
    )
  }

  /**
   * Configures `slogging` logger.
   */
  private def configureLogging(): Unit = {
    PrintLoggerFactory.formatter = new DefaultPrefixFormatter(true, false, true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = LogLevel.INFO
  }
}
