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

import cats.Parallel
import cats.effect._
import cats.effect.syntax.effect._
import cats.syntax.applicative._
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.softwaremill.sttp.SttpBackend
import fluence.ethclient.EthClient
import fluence.node.config.{MasterConfig, NodeConfig}
import fluence.node.docker.{DockerIO, DockerImage, DockerNetwork}
import fluence.node.eth._
import fluence.node.workers._
import fluence.node.workers.tendermint.config.WorkerConfigWriter
import fluence.node.workers.tendermint.config.WorkerConfigWriter.WorkerConfigPaths
import slogging.LazyLogging

import scala.language.higherKinds

/**
 * Represents a MasterNode process. Takes cluster forming events from Ethereum, and spawns new Workers to serve them.
 *
 * @param nodeConfig Tendermint/Fluence master node config
 * @param nodeEth Ethereum adapter
 * @param pool Workers pool to launch workers in
 * @param rootPath MasterNode's working directory, usually /master
 */
case class MasterNode[F[_]: ConcurrentEffect: LiftIO](
  nodeConfig: NodeConfig,
  nodeEth: NodeEth[F],
  pool: WorkersPool[F],
  codeManager: CodeManager[F],
  rootPath: Path,
  masterNodeContainerId: Option[String]
) extends slogging.LazyLogging {

  /**
   * Downloads code from Swarm
   *
   * @param codeManager Manager that downloads the code from Swarm
   * @return original App and WorkerConfigPaths along with downloaded code Path
   */
  private def downloadCode(
    codeManager: CodeManager[F],
    app: state.App,
    paths: WorkerConfigPaths
  ): F[Path] = codeManager.prepareCode(CodePath(app.storageHash), paths.workerPath)

  /**
   * Generates WorkerParams case class from app, config paths and downloaded code path
   *
   * @param workerImage Docker image to use to run a Worker
   * @param masterNodeContainerId Docker container id of the current Fluence node, used to import volumes from it
   * @return
   */
  private def buildWorkerParams(
    workerImage: DockerImage,
    masterNodeContainerId: Option[String],
    app: state.App,
    paths: WorkerConfigPaths,
    codePath: Path
  ) = WorkerParams(
    app.id,
    app.cluster.currentWorker,
    paths.workerPath.toString,
    codePath.toAbsolutePath.toString,
    masterNodeContainerId,
    workerImage
  )

  private def runWorker(params: WorkerParams) =
    for {
      _ <- IO(logger.info("Running worker `{}`", params)).to[F]
      newly <- pool.run(params)
      _ <- IO(logger.info(s"Worker started (newly=$newly) {}", params)).to[F]
    } yield ()

  /**
   * Runs app worker on a pool
   * TODO check that the worker is not yet running
   *
   * @param app App description
   */
  def runAppWorker(app: eth.state.App): F[Unit] =
    for {
      paths <- WorkerConfigWriter.resolveWorkerConfigPaths(app, rootPath)
      code <- downloadCode(codeManager, app, paths)
      _ <- WorkerConfigWriter.writeConfigs(app, paths)

      params = buildWorkerParams(nodeConfig.workerImage, masterNodeContainerId, app, paths, code)

      _ <- runWorker(params)
    } yield ()

  /**
   * Runs the appropriate effect for each incoming NodeEthEvent, keeping it untouched
   */
  val handleEthEvent: fs2.Pipe[F, NodeEthEvent, NodeEthEvent] =
    _.evalTap {
      case RunAppWorker(app) ⇒
        runAppWorker(app)

      case RemoveAppWorker(appId) ⇒
        pool.get(appId).flatMap {
          case Some(w) ⇒
            w.stop.attempt.map { stopped =>
              logger.info(s"Stopped: ${w.description} => $stopped")
            }
          case _ ⇒ ().pure[F]
        }

      case DropPeerWorker(appId, vk) ⇒
        pool.get(appId).flatMap {
          case Some(w) ⇒
            w.control.dropPeer(vk)
          case None ⇒ ().pure[F]
        }

      case NewBlockReceived(block) ⇒
        ().pure[F]
    }

  /**
   * Runs master node and starts listening for AppDeleted event in different threads,
   * then joins the threads and returns back exit code from master node
   */
  val run: IO[ExitCode] =
    nodeEth.nodeEvents
      .evalTap(ev ⇒ Sync[F].delay(logger.debug("Got NodeEth event: " + ev)))
      .through(handleEthEvent)
      .drain
      .compile
      .drain
      .toIO
      .attempt
      .map {
        case Left(err) ⇒
          logger.error("Execution failed")
          err.printStackTrace(System.err)
          ExitCode.Error

        case Right(_) ⇒
          logger.info("Execution finished")
          ExitCode.Success
      }
}

object MasterNode extends LazyLogging {

  /**
   * Makes the MasterNode resource for the given config
   *
   * @param masterConfig MasterConfig
   * @param nodeConfig NodeConfig
   * @param rootPath Master's root path
   * @param sttpBackend HTTP client implementation
   * @param P Parallel instance, used for Workers
   * @return Prepared [[MasterNode]], then see [[MasterNode.run]]
   */
  def make[F[_]: ConcurrentEffect: LiftIO: ContextShift: Timer, G[_]](
    masterConfig: MasterConfig,
    nodeConfig: NodeConfig,
    rootPath: Path
  )(implicit sttpBackend: SttpBackend[F, Nothing], P: Parallel[F, G]): Resource[F, MasterNode[F]] =
    for {
      ethClient ← EthClient.make[F](Some(masterConfig.ethereum.uri))

      pool ← WorkersPool.make()

      nodeEth ← NodeEth[F](nodeConfig.validatorKey.toByteVector, ethClient, masterConfig.contract)

      codeManager ← Resource.liftF(CodeManager[F](masterConfig.swarm))
    } yield
      MasterNode[F](
        nodeConfig,
        nodeEth,
        pool,
        codeManager,
        rootPath,
        masterConfig.masterContainerId
      )
}
