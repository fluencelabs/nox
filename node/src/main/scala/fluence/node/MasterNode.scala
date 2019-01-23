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

import cats.effect.ExitCase.{Canceled, Completed, Error}
import cats.effect._
import fluence.ethclient.helpers.Web3jConverters
import fluence.node.config.NodeConfig
import fluence.node.eth.{App, FluenceContract}
import fluence.node.tendermint.config.WorkerConfigWriter
import fluence.node.tendermint.config.WorkerConfigWriter.WorkerConfigPaths
import fluence.node.workers._

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

  /**
   * Downloads code from Swarm
   * @param codeManager Manager that downloads the code from Swarm
   * @return original App and WorkerConfigPaths along with downloaded code Path
   */
  private def downloadCode(
    codeManager: CodeManager[IO]
  ): fs2.Pipe[IO, (App, WorkerConfigPaths), (App, WorkerConfigPaths, Path)] =
    _.evalMap {
      case (app, paths) =>
        codeManager.prepareCode(CodePath(app.storageHash), paths.workerPath).map { codePath =>
          (app, paths, codePath)
        }
    }

  /**
   * Generates WorkerParams case class from app, config paths and downloaded code path
   * @param workerImage Docker image to use to run a Worker
   * @param masterNodeContainerId Docker container id of the current Fluence node, used to import volumes from it
   * @return
   */
  private def buildWorkerParams(
    workerImage: WorkerImage,
    masterNodeContainerId: Option[String]
  ): fs2.Pipe[IO, (App, WorkerConfigPaths, Path), WorkerParams] =
    _.map {
      case (app, paths, codePath) =>
        WorkerParams(
          app.appId,
          app.cluster.currentWorker,
          paths.workerPath.toString,
          codePath.toAbsolutePath.toString,
          masterNodeContainerId,
          workerImage
        )
    }

  /**
   * Runs MasterNode. Returns when contract.getAllNodeClusters is exhausted
   * TODO: add a way to cleanup, e.g. unsubscribe and stop
   */
  private val runMasterNode: IO[ExitCode] =
    contract
      .getAllNodeApps(nodeConfig.validatorKey.toBytes32)
      .evalTap[IO](app => IO { logger.info("This node will host app '{}'", app.appIdHex) })
      .through(WorkerConfigWriter.resolveWorkerConfigPaths(rootPath))
      .through(downloadCode(codeManager))
      .through(WorkerConfigWriter.writeConfigs())
      .through(buildWorkerParams(nodeConfig.workerImage, masterNodeContainerId))
      .evalMap[IO, Unit](
        params ⇒
          for {
            _ <- IO(logger.info("Running worker `{}`", params))
            newly <- pool.run(params)
            _ <- IO(logger.info(s"Worker started (newly=$newly) {}", params))
          } yield ()
      )
      .drain // drop the results, so that demand on events is always provided
      .onFinalize(IO(logger.info("runMasterNode finalized")))
      .compile // Compile to a runnable, in terms of effect IO
      .drain // Switch to IO[Unit]
      .map(_ ⇒ ExitCode.Success)
      .guaranteeCase {
        case Error(e) =>
          IO {
            logger.error(s"runMasterNode error: $e")
            e.printStackTrace(System.err)
          }
        case Canceled => IO(logger.error("runMasterNode was canceled"))
        case Completed => IO(logger.info("runMasterNode finished gracefully"))
      }

  /**
   * Starts listening for AppDeleted event and deletes a worker for that app if it exists. Does nothing otherwise.
   */
  private val listenForDeletion: IO[Unit] =
    contract.getAppDeleted
      .map(Web3jConverters.bytes32ToBinary)
      .evalMap(pool.stopWorkerForApp)
      .drain
      .compile
      .drain

  /**
   * Runs master node and starts listening for AppDeleted event in different threads,
   * then joins the threads and returns back exit code from master node
   */
  val run: IO[ExitCode] =
    for {
      node <- Concurrent[IO].start(runMasterNode)
      appDelete <- Concurrent[IO].start(listenForDeletion)
      exitCode <- node.join
      _ <- appDelete.cancel
    } yield exitCode
}
