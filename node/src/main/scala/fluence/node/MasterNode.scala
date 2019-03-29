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
import fluence.effects.docker.DockerIO
import fluence.effects.ethclient.EthClient
import fluence.node.config.{MasterConfig, NodeConfig}
import fluence.node.eth._
import fluence.node.workers._
import fluence.node.workers.tendermint.config.ConfigTemplate
import slogging.LazyLogging

import scala.language.higherKinds

/**
 * Represents a MasterNode process. Takes cluster forming events from Ethereum, and spawns new Workers to serve them.
 *
 * @param nodeConfig Tendermint/Fluence master node config
 * @param configTemplate Template for worker's configuration
 * @param nodeEth Ethereum adapter
 * @param pool Workers pool to launch workers in
 * @param codeManager To load the code from, usually backed with Swarm
 * @param rootPath MasterNode's working directory, usually /master
 * @param masterNodeContainerId Docker Container ID for this process, to import Docker volumes from
 */
case class MasterNode[F[_]: ConcurrentEffect: LiftIO](
  nodeConfig: NodeConfig,
  configTemplate: ConfigTemplate,
  nodeEth: NodeEth[F],
  pool: WorkersPool[F],
  codeManager: CodeManager[F],
  rootPath: Path,
  masterNodeContainerId: Option[String]
) extends slogging.LazyLogging {

  private def runWorker(params: WorkerParams) =
    for {
      _ <- IO(logger.info("Worker starting: `{}`", params)).to[F]
      status <- pool.run(params)
      _ <- IO(logger.info(s"Worker started, status: $status `{}`", params)).to[F]
    } yield ()

  /**
   * All app worker's data is stored here. Currently the folder is never purged
   */
  private def resolveAppPath(app: eth.state.App): F[Path] =
    IO(rootPath.resolve("app-" + app.id + "-" + app.cluster.currentWorker.index)).to[F]

  /**
   * Create directory to hold Tendermint config & data for a specific app (worker)
   *
   * @param appPath Path containing all configs & data for a specific app
   * @return Path to Tendermint home ($TMHOME) directory
   */
  private def makeTendermintPath(appPath: Path): F[Path] =
    for {
      tendermintPath ← IO(appPath.resolve("tendermint")).to[F]
      _ ← IO(Files.createDirectories(tendermintPath)).to[F]
    } yield tendermintPath

  /**
   * Create directory to hold app code downloaded from Swarm
   *
   * @param appPath Path containing all configs & data for a specific app
   * @return Path to `vmcode` directory
   */
  private def makeVmCodePath(appPath: Path): F[Path] =
    for {
      vmCodePath ← IO(appPath.resolve("vmcode")).to[F]
      _ ← IO(Files.createDirectories(vmCodePath)).to[F]
    } yield vmCodePath

  /**
   * Runs app worker on a pool
   * TODO check that the worker is not yet running
   *
   * @param app App description
   */
  def runAppWorker(app: eth.state.App): F[Unit] =
    for {
      appPath ← resolveAppPath(app)
      tendermintPath ← makeTendermintPath(appPath)
      vmCodePath ← makeVmCodePath(appPath)

      // TODO: in general, worker/vm is responsible about downloading the code during resource creation, isn't it?
      // we take output to substitute test folder in tests
      code <- codeManager.prepareCode(CodePath(app.storageHash), vmCodePath)

      _ <- runWorker(
        WorkerParams(
          app,
          tendermintPath,
          code,
          masterNodeContainerId,
          nodeConfig.workerDockerConfig,
          nodeConfig.tmDockerConfig,
          configTemplate
        )
      )
    } yield ()

  def handleEthEvent(event: NodeEthEvent) = event match {
    case RunAppWorker(app) ⇒
      runAppWorker(app)

    case RemoveAppWorker(appId) ⇒
      pool.get(appId).flatMap {
        case Some(w) ⇒
          w.remove.attempt.map { removed =>
            logger.info(s"Removed: ${w.description} => $removed")
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
   * Runs the appropriate effect for each incoming NodeEthEvent, keeping it untouched
   */
  val handleEthEventPipe: fs2.Pipe[F, NodeEthEvent, NodeEthEvent] =
    _.evalTap(handleEthEvent)

  /**
   * Runs master node and starts listening for AppDeleted event in different threads,
   * then joins the threads and returns back exit code from master node
   */
  val run: IO[ExitCode] =
    nodeEth.nodeEvents.bufferAll
      .evalTap(ev ⇒ Sync[F].delay(logger.debug("Got NodeEth event: " + ev)))
      .parEvalMap(1024)(e => handleEthEvent(e).as(e))
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
  def make[F[_]: ConcurrentEffect: LiftIO: ContextShift: Timer: DockerIO, G[_]](
    masterConfig: MasterConfig,
    nodeConfig: NodeConfig,
    rootPath: Path
  )(implicit sttpBackend: SttpBackend[F, Nothing], P: Parallel[F, G]): Resource[F, MasterNode[F]] =
    for {
      ethClient ← EthClient.make[F](Some(masterConfig.ethereum.uri))

      _ = logger.debug("-> going to create a pool")

      // TODO wrap Paths.get somehow?
      pool ← DockerWorkersPool
        .make(masterConfig.ports.minPort, masterConfig.ports.maxPort, Paths.get(masterConfig.rootPath))

      _ = logger.debug("-> going to create nodeEth")

      nodeEth ← NodeEth[F](nodeConfig.validatorKey.toByteVector, ethClient, masterConfig.contract)

      codeManager ← Resource.liftF(CodeManager[F](masterConfig.swarm))
      configTemplate ← Resource.liftF(ConfigTemplate[F](rootPath, masterConfig.tendermintConfig))
    } yield
      MasterNode[F](
        nodeConfig,
        configTemplate,
        nodeEth,
        pool,
        codeManager,
        rootPath,
        masterConfig.masterContainerId
      )
}
