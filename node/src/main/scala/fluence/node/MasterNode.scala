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

import cats.effect._
import cats.effect.syntax.effect._
import cats.syntax.compose._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Applicative, Monad, Parallel}
import fluence.bp.uploading.BlockUploading
import fluence.codec.PureCodec
import fluence.effects.docker.DockerIO
import fluence.effects.ethclient.EthClient
import fluence.effects.ipfs.IpfsUploader
import fluence.effects.kvstore.RocksDBStore
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.sttp.{SttpEffect, SttpStreamEffect}
import fluence.effects.{Backoff, EffectError}
import fluence.kad.Kademlia
import fluence.log.{Log, LogFactory}
import fluence.node.code.CodeCarrier
import fluence.node.config.storage.RemoteStorageConfig
import fluence.node.config.{MasterConfig, NodeConfig}
import fluence.node.eth._
import fluence.node.workers.tendermint.config.ConfigTemplate
import fluence.node.workers.{WorkerDocker, WorkerFiles, WorkersPorts}
import fluence.statemachine.api.command.PeersControl
import fluence.worker.WorkerStage
import fluence.worker.eth.EthApp

import scala.language.{higherKinds, postfixOps}

/**
 * Represents a MasterNode process. Takes cluster forming events from Ethereum, and spawns new Workers to serve them.
 *
 * @param nodeEth Ethereum adapter
 * @param pool Workers pool to launch workers in
 */
case class MasterNode[F[_]: ConcurrentEffect: LiftIO: LogFactory, C](
  nodeEth: NodeEth[F],
  pool: MasterPool.Type[F]
)(implicit backoff: Backoff[EffectError]) {

  /**
   * Runs app worker on a pool
   *
   * @param app App description
   */
  def runAppWorker(app: EthApp): F[Unit] =
    for {
      implicit0(log: Log[F]) ← LogFactory[F].init("app", app.id.toString)
      _ ← log.info("Running worker")
      _ <- pool.run(app)
    } yield ()

  /**
   * Runs the appropriate effect for each incoming NodeEthEvent, keeping it untouched
   */
  def handleEthEvent(implicit log: Log[F]): fs2.Pipe[F, NodeEthEvent, NodeEthEvent] =
    _.evalTap {
      case RunAppWorker(app) ⇒
        runAppWorker(app)

      case RemoveAppWorker(appId) ⇒
        pool.get(appId).semiflatMap(_.destroy()).value.void

      case DropPeerWorker(appId, vk) ⇒
        Log[F].scope("app" -> appId.toString, "key" -> vk.toHex) { implicit log: Log[F] =>
          pool
            .getCompanion[PeersControl[F]](appId)
            .semiflatMap(
              _.dropPeer(vk).valueOr(e ⇒ log.error(s"Unexpected error while dropping peer", e))
            )
            .valueOr((st: WorkerStage) ⇒ log.error(s"No available worker for $appId: it's on stage $st"))
            .void
        }

      case NewBlockReceived(_) ⇒
        Applicative[F].unit

      case ContractAppsLoaded ⇒
        Applicative[F].unit
    }

  /**
   * Runs master node and starts listening for AppDeleted event in different threads,
   * then joins the threads and returns back exit code from master node
   */
  def run(implicit log: Log[F]): IO[ExitCode] =
    nodeEth.nodeEvents
      .evalTap(ev ⇒ log.debug("Got NodeEth event: " + ev))
      .through(handleEthEvent)
      .compile
      .drain
      .toIO
      .attempt
      .flatMap {
        case Left(err) ⇒
          log.error("Execution failed", err) as ExitCode.Error toIO

        case Right(_) ⇒
          log.info("Execution finished") as ExitCode.Success toIO
      }
}

object MasterNode {

  // TODO drop unused args
  /**
   * Makes the MasterNode resource for the given config
   *
   * @param masterConfig MasterConfig
   * @param nodeConfig NodeConfig
   * @return Prepared [[MasterNode]], then see [[MasterNode.run]]
   */
  def make[F[_]: ConcurrentEffect: ContextShift: Timer: Log: LogFactory: Parallel: DockerIO: SttpStreamEffect, C](
    masterConfig: MasterConfig,
    nodeConfig: NodeConfig,
    kademlia: Kademlia[F, C]
  )(
    implicit
    backoff: Backoff[EffectError]
  ): Resource[F, MasterNode[F, C]] =
    for {
      ethClient ← EthClient.make[F](Some(masterConfig.ethereum.uri))

      _ ← Log.resource[F].debug("-> going to create nodeEth")

      nodeEth ← NodeEth[F](nodeConfig.validatorKey.toByteVector, ethClient, masterConfig.contract)

      rootPath <- Resource.liftF(IO(Paths.get(masterConfig.rootPath).toAbsolutePath).to[F])

      workersPool ← workersPool(masterConfig, rootPath)

    } yield MasterNode[F, C](nodeEth, workersPool)

  private def ipfsUploader[F[_]: Monad](conf: RemoteStorageConfig)(implicit sttp: SttpStreamEffect[F]) =
    IpfsUploader[F](conf.ipfs.address, conf.enabled, conf.ipfs.readTimeout)

  private def workersPool[F[_]: ConcurrentEffect: Parallel: ContextShift: Timer: Log: DockerIO: SttpStreamEffect: SttpEffect](
    conf: MasterConfig,
    rootPath: Path
  )(implicit backoff: Backoff[EffectError]) = {
    // TODO: hide codecs somewhere, incapsulate
    // TODO use better serialization, check for errors
    implicit val stringCodec: PureCodec[String, Array[Byte]] = PureCodec
      .liftB[String, Array[Byte]](_.getBytes(), bs ⇒ new String(bs))

    implicit val longCodec: PureCodec[Array[Byte], Long] = PureCodec[Array[Byte], String] andThen PureCodec
      .liftB[String, Long](_.toLong, _.toString)

    implicit val shortCodec: PureCodec[Array[Byte], Short] = PureCodec[Array[Byte], String] andThen PureCodec
      .liftB[String, Short](_.toShort, _.toString)

    for {
      portsStore ← RocksDBStore.make[F, Long, Short](rootPath.resolve("p2p-ports-db").toString)
      ports ← WorkersPorts.make[F](conf.ports.minPort, conf.ports.maxPort, portsStore)
      workerDocker = (app: EthApp) ⇒
        WorkerDocker[F](
          app,
          conf.masterContainerId,
          conf.tendermint,
          conf.worker,
          conf.dockerStopTimeout,
          conf.logLevel
        )
      codeCarrier ← Resource.pure(CodeCarrier[F](conf.remoteStorage))
      workerFiles = WorkerFiles(rootPath, codeCarrier)
      receiptStorage = (app: EthApp) ⇒ ReceiptStorage.local(app.id, rootPath)
      blockUploading ← BlockUploading[F](conf.blockUploadingEnabled, ipfsUploader(conf.remoteStorage))
      configTemplate ← Resource.liftF(ConfigTemplate[F](rootPath, conf.tendermintConfig))

      pool <- Resource.liftF(
        MasterPool[F](ports, workerDocker, workerFiles, receiptStorage, blockUploading, conf.websocket, configTemplate)
      )
    } yield pool
  }
}
