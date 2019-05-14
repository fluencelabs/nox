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
import java.nio.ByteBuffer
import java.nio.file._

import cats.Parallel
import cats.data.EitherT
import cats.effect._
import cats.effect.syntax.effect._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.softwaremill.sttp.SttpBackend
import fluence.effects.Backoff
import fluence.effects.castore.StoreError
import fluence.effects.docker.DockerIO
import fluence.effects.ethclient.EthClient
import fluence.effects.ipfs.{IpfsClient, IpfsStore}
import fluence.effects.swarm.{SwarmClient, SwarmStore}
import fluence.node.code.{CodeCarrier, LocalCodeCarrier, PolyStore, RemoteCodeCarrier}
import fluence.node.config.storage.RemoteStorageConfig
import fluence.node.config.{MasterConfig, NodeConfig}
import fluence.node.eth._
import fluence.node.eth.state.StorageType
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
 * @param codeCarrier To load the code from, usually backed with Swarm
 * @param rootPath MasterNode's working directory, usually /master
 * @param masterNodeContainerId Docker Container ID for this process, to import Docker volumes from
 */
case class MasterNode[F[_]: ConcurrentEffect: LiftIO](
  nodeConfig: NodeConfig,
  configTemplate: ConfigTemplate,
  nodeEth: NodeEth[F],
  pool: WorkersPool[F],
  codeCarrier: CodeCarrier[F],
  rootPath: Path,
  masterNodeContainerId: Option[String]
) extends slogging.LazyLogging {

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

  def prepareWorkerParams(app: eth.state.App): F[WorkerParams] =
    for {
      appPath ← resolveAppPath(app)
      tendermintPath ← makeTendermintPath(appPath)
      vmCodePath ← makeVmCodePath(appPath)

      // TODO: Move description of the code preparation to Worker; it should be Worker's responsibility
      code <- codeCarrier.carryCode(app.code, vmCodePath)
    } yield
      WorkerParams(
        app,
        tendermintPath,
        code,
        masterNodeContainerId,
        nodeConfig.workerDockerConfig,
        nodeConfig.tmDockerConfig,
        configTemplate
      )

  /**
   * Runs app worker on a pool
   *
   * @param app App description
   */
  def runAppWorker(app: eth.state.App): F[Unit] =
    for {
      _ <- IO(logger.info("Running worker for id `{}`", app.id)).to[F]
      _ <- pool.run(app.id, prepareWorkerParams(app))
    } yield ()

  /**
   * Runs the appropriate effect for each incoming NodeEthEvent, keeping it untouched
   */
  val handleEthEvent: fs2.Pipe[F, NodeEthEvent, NodeEthEvent] =
    _.evalTap {
      case RunAppWorker(app) ⇒
        runAppWorker(app)

      case RemoveAppWorker(appId) ⇒
        pool.withWorker(appId, _.remove).void

      case DropPeerWorker(appId, vk) ⇒
        pool.withWorker(appId, _.withServices_(_.control)(_.dropPeer(vk))).void

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
   * @param sttpBackend HTTP client implementation
   * @return Prepared [[MasterNode]], then see [[MasterNode.run]]
   */
  def make[F[_]: ConcurrentEffect: LiftIO: ContextShift: Timer, G[_]](
    masterConfig: MasterConfig,
    nodeConfig: NodeConfig,
    pool: WorkersPool[F]
  )(
    implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]]
  ): Resource[F, MasterNode[F]] =
    for {
      ethClient ← EthClient.make[F](Some(masterConfig.ethereum.uri))

      _ = logger.debug("-> going to create nodeEth")

      nodeEth ← NodeEth[F](nodeConfig.validatorKey.toByteVector, ethClient, masterConfig.contract)

      codeCarrier ← Resource.pure(codeCarrier[F](masterConfig.remoteStorage))

      rootPath <- Resource.liftF(IO(Paths.get(masterConfig.rootPath).toAbsolutePath).to[F])

      configTemplate ← Resource.liftF(ConfigTemplate[F](rootPath, masterConfig.tendermintConfig))
    } yield
      MasterNode[F](
        nodeConfig,
        configTemplate,
        nodeEth,
        pool,
        codeCarrier,
        rootPath,
        masterConfig.masterContainerId
      )

  def codeCarrier[F[_]: Sync: ContextShift: Concurrent: Timer: LiftIO](
    config: RemoteStorageConfig
  )(implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]]): CodeCarrier[F] =
    if (config.enabled) {
      implicit val b: Backoff[StoreError] = Backoff.default
      val swarmStore = SwarmStore[F](config.swarm.address)
      val ipfsStore = IpfsStore[F](config.ipfs.address)
      val polyStore = new PolyStore[F]({
        case StorageType.Swarm => swarmStore
        case StorageType.Ipfs => ipfsStore
      })
      new RemoteCodeCarrier[F](polyStore)
    } else {
      new LocalCodeCarrier[F]()
    }
}
