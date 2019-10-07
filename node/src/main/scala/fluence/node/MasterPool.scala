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

import java.nio.file.Path

import cats.{Monad, Parallel}
import cats.effect.{ConcurrentEffect, ContextShift, LiftIO, Resource, Timer}
import cats.syntax.apply._
import cats.syntax.compose._
import cats.syntax.applicative._
import fluence.bp.api.{BlockStream, DialPeers}
import fluence.bp.uploading.BlockUploading
import fluence.codec.PureCodec
import fluence.effects.{Backoff, EffectError}
import fluence.effects.docker.DockerIO
import fluence.effects.ipfs.IpfsUploader
import fluence.effects.kvstore.RocksDBStore
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.sttp.{SttpEffect, SttpStreamEffect}
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.websocket.WebsocketConfig
import fluence.log.Log
import fluence.node.code.CodeCarrier
import fluence.node.config.MasterConfig
import fluence.node.workers.{WorkerBlockManifests, WorkerDocker, WorkerFiles, WorkerP2pConnectivity, WorkersPorts}
import fluence.node.workers.tendermint.DockerTendermint
import fluence.node.workers.tendermint.config.ConfigTemplate
import fluence.statemachine.api.command.{PeersControl, ReceiptBus}
import fluence.statemachine.docker.DockerStateMachine
import fluence.worker.{Worker, WorkerContext, WorkerResource, WorkersPool}
import fluence.worker.eth.EthApp
import fluence.worker.responder.WorkerResponder
import shapeless._

import scala.language.{higherKinds, reflectiveCalls}

object MasterPool {

  type Resources[F[_]] = WorkerFiles.Paths[F] :: WorkersPorts.P2pPort[F] :: HNil
  type Companions[F[_]] = PeersControl[F] :: WorkerResponder[F] :: HNil

  type Type[F[_]] = WorkersPool[F, Resources[F], Companions[F]]

  def resources[F[_]: Monad: Timer](
    app: EthApp,
    ports: WorkersPorts[F],
    files: WorkerFiles[F]
  )(implicit backoff: Backoff[EffectError]): WorkerResource[F, Resources[F]] =
    (
      files.workerResource(app),
      ports.workerResource(app.id)
    ).mapN((f, p) ⇒ f :: p :: HNil)

  def docker[F[_]: ConcurrentEffect: Parallel: ContextShift: Timer: Log: DockerIO: SttpStreamEffect](
    conf: MasterConfig,
    appReceiptStorage: Long ⇒ Resource[F, ReceiptStorage[F]],
    rootPath: Path
  )(implicit backoff: Backoff[EffectError]): Resource[F, Type[F]] = {
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
          s"app_${app.id}_${app.cluster.currentWorker.index}",
          s"sm_${app.id}_${app.cluster.currentWorker.index}",
          s"bp_${app.id}_${app.cluster.currentWorker.index}",
          conf.masterContainerId,
          conf.tendermint,
          conf.worker,
          conf.dockerStopTimeout,
          conf.logLevel
        )

      codeCarrier ← Resource.pure(CodeCarrier[F](conf.remoteStorage))
      workerFiles = WorkerFiles(rootPath, codeCarrier)

      // TODO local storage
      receiptStorage = (app: EthApp) ⇒ appReceiptStorage(app.id)
      blockUploading ← BlockUploading[F](
        conf.blockUploadingEnabled,
        IpfsUploader[F](
          conf.remoteStorage.ipfs.address,
          conf.remoteStorage.enabled,
          conf.remoteStorage.ipfs.readTimeout
        )
      )
      configTemplate ← Resource.liftF(ConfigTemplate[F](rootPath, conf.tendermintConfig))

      pool <- makeDocker(
        ports,
        workerDocker,
        workerFiles,
        receiptStorage,
        blockUploading,
        conf.websocket,
        configTemplate
      )

    } yield pool
  }

  private def makeDocker[F[_]: Parallel: Timer: ConcurrentEffect: DockerIO: LiftIO: ContextShift](
    ports: WorkersPorts[F],
    workerDocker: EthApp ⇒ Resource[F, WorkerDocker],
    files: WorkerFiles[F],
    // TODO make them companions/resources?
    receiptStorage: EthApp ⇒ Resource[F, ReceiptStorage[F]],
    blockUploading: BlockUploading[F],
    websocketConfig: WebsocketConfig,
    configTemplate: ConfigTemplate
  )(implicit backoff: Backoff[EffectError], sttp: SttpEffect[F], log: Log[F]): Resource[F, Type[F]] = {

    type W0 = Worker[F, Companions[F]]

    def worker(app: EthApp)(implicit log: Log[F]): Resources[F] ⇒ Resource[F, W0] =
      res ⇒
        for {
          wd ← workerDocker(app)

          p2pPort ← Resource.liftF(
            res.select[WorkersPorts.P2pPort[F]].port
          )

          paths = res.select[WorkerFiles.Paths[F]]

          codePath ← Resource.liftF(paths.code)

          machine ← DockerStateMachine.make[F](
            wd.machine.name,
            wd.network,
            wd.machine.docker.limits,
            wd.machine.docker.image,
            wd.machine.docker.environment,
            wd.logLevel,
            codePath.toAbsolutePath.toString,
            wd.masterContainerId,
            wd.stopTimeout
          )

          producer ← DockerTendermint
            .make[F](app, paths.tendermint, configTemplate, wd, p2pPort, websocketConfig)

          _ ← WorkerP2pConnectivity.make[F](app.id, producer.command[DialPeers[F]], app.cluster.workers)

          manifests ← WorkerBlockManifests.make[F](receiptStorage(app))

          _ ← blockUploading.start(
            app.id,
            manifests.receiptStorage,
            producer.command[BlockStream[F, Block]],
            machine.command[ReceiptBus[F]],
            manifests.onUploaded
          )

          responder ← WorkerResponder.make[F, producer.Commands, Block](producer, machine)

        } yield Worker(
          app.id,
          machine,
          producer,
          machine.command[PeersControl[F]] :: responder :: HNil
        )

    WorkersPool.make[F, Resources[F], Companions[F]] { (app, l) ⇒
      implicit val log: Log[F] = l
      WorkerContext(
        app,
        resources(app, ports, files),
        worker(app)
      )
    }
  }

}
