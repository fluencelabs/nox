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

package fluence.node
import java.nio.file._

import cats.effect.{Concurrent, ConcurrentEffect, ExitCode, IO}
import fluence.ethclient.helpers.Web3jConverters
import fluence.node.config.NodeConfig
import fluence.node.eth.{App, FluenceContract}
import fluence.node.tendermint.config.TendermintConfig
import fluence.node.workers._
import fs2.Pipe

/**
 * Represents a MasterNode process. Takes cluster forming events from Ethereum, and spawns new Workers to serve them.
 *
 * @param nodeConfig Tendermint/Fluence master node config
 * @param contract Contract to interact with
 * @param pool Workers pool to launch workers in
 * @param rootPath MasterNode's working directory, usually /master
 * @param ce Concurrent effect, used to subscribe to Ethereum events
 */
case class MasterNode(
  nodeConfig: NodeConfig,
  contract: FluenceContract,
  pool: WorkersPool[IO],
  codeManager: CodeManager[IO],
  rootPath: Path,
  masterNodeContainerId: Option[String]
)(implicit ce: ConcurrentEffect[IO])
    extends slogging.LazyLogging {

  private val prepareWorkerParams: Pipe[IO, App, WorkerParams] = {
    TendermintConfig.prepareWorkerParams(
      nodeConfig.validatorKey.toBytes32,
      nodeConfig.workerImage,
      rootPath,
      masterNodeContainerId,
      codeManager
    )
  }

  /**
   * Runs MasterNode. Returns when contract.getAllNodeClusters is exhausted
   * TODO: add a way to cleanup, e.g. unsubscribe and stop
   */
  private val masterNode: IO[ExitCode] =
    contract
      .getAllNodeApps(nodeConfig.validatorKey.toBytes32)
      .through(prepareWorkerParams)
      .evalMap { params ⇒
        logger.info("Running worker `{}`", params)

        pool.run(params).map(newlyAdded ⇒ logger.info(s"worker run (newly=$newlyAdded) {}", params))
      }
      .drain // drop the results, so that demand on events is always provided
      .onFinalize(IO(logger.info("subscription finalized")))
      .compile // Compile to a runnable, in terms of effect IO
      .drain // Switch to IO[Unit]
      .map(_ ⇒ ExitCode.Success)

  /**
   * Starts listening for AppDeleted event and deletes a worker for that app if it exists. Does nothing otherwise.
   */
  private val listenForDeletion: IO[Unit] =
    contract.getAppDeleted
      .evalMap(pool.stopWorkerForApp)
      .drain
      .compile
      .drain

  /**
   * Runs master node and starts listening for AppDeleted event in different threads,
   * then joins the threads and returns back exit code from master node
   */
  val run: IO[ExitCode] = for {
    node <- Concurrent[IO].start(masterNode)
    appDelete <- Concurrent[IO].start(listenForDeletion)
    exitCode <- node.join
    _ <- appDelete.join
  } yield exitCode
}
