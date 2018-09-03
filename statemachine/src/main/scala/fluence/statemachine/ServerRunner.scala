/*
 * Copyright (C) 2018  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.statemachine

import cats.effect.concurrent.MVar
import cats.effect.{ExitCode, IO, IOApp}
import com.github.jtendermint.jabci.socket.TSocket
import fluence.statemachine.contract.ClientRegistry
import fluence.statemachine.state._
import fluence.statemachine.tree.TreeNode
import fluence.statemachine.tx.{TxDuplicateChecker, TxParser, TxProcessor, VmOperationInvoker}
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
      checkTxDuplicateChecker = new TxDuplicateChecker[IO](stateHolder.mempoolState)
      deliverTxDuplicateChecker = new TxDuplicateChecker(mutableConsensusState.getRoot)
      txProcessor = new TxProcessor(mutableConsensusState, vmInvoker)

      committer = new Committer[IO](stateHolder, vmInvoker)

      abciHandler = new AbciHandler(
        committer,
        queryProcessor,
        txParser,
        checkTxDuplicateChecker,
        deliverTxDuplicateChecker,
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
