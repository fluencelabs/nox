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

import cats.{Monad, Parallel}
import cats.effect.{ConcurrentEffect, ContextShift, LiftIO, Resource, Timer}
import cats.syntax.apply._
import fluence.bp.api.DialPeers
import fluence.bp.uploading.BlockUploading
import fluence.effects.{Backoff, EffectError}
import fluence.effects.docker.DockerIO
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.rpc.websocket.WebsocketConfig
import fluence.log.Log
import fluence.node.workers.{WorkerBlockManifests, WorkerDocker, WorkerFiles, WorkerP2pConnectivity, WorkersPorts}
import fluence.node.workers.tendermint.DockerTendermint
import fluence.node.workers.tendermint.config.ConfigTemplate
import fluence.statemachine.api.command.{PeersControl, ReceiptBus}
import fluence.statemachine.docker.DockerStateMachine
import fluence.worker.{Worker, WorkerContext, WorkerResource, WorkersPool}
import fluence.worker.eth.EthApp
import fluence.worker.responder.WorkerResponder
import shapeless._

import scala.language.higherKinds

object MasterPool {

  case class CodePath(path: String)

  type Resources[F[_]] = WorkerFiles.Paths[F] :: WorkersPorts.P2pPort[F] :: HNil
  type Companions[F[_]] = PeersControl[F] ::  WorkerResponder[F] :: HNil

  def resources[F[_]: Monad: Timer](
    app: EthApp,
    ports: WorkersPorts[F],
    files: WorkerFiles[F]
  )(implicit backoff: Backoff[EffectError]): WorkerResource[F, Resources[F]] =
    (
    files(app),
      ports.workerResource(app.id)
      ).mapN(_ :: _ :: HNil)

  def apply[F[_]: Parallel: Timer: ConcurrentEffect: DockerIO: LiftIO: ContextShift](
    ports: WorkersPorts[F],
    workerDocker: EthApp ⇒ Resource[F, WorkerDocker],
    files: WorkerFiles[F],

    // TODO make them companions/resources?
    receiptStorage: Resource[F, ReceiptStorage[F]],
    blockUploading: BlockUploading[F],
    websocketConfig: WebsocketConfig,
    configTemplate: ConfigTemplate
  )(implicit backoff: Backoff[EffectError], sttp: SttpEffect[F]): F[WorkersPool[F, Resources[F], Companions[F]]] = {

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
            wd.machine.limits,
            wd.machine.image,
            wd.logLevel,
            wd.machine.environment,
            codePath.toAbsolutePath.toString,
            wd.masterContainerId,
            wd.stopTimeout
          )

          producer ← DockerTendermint
            .make[F](app, paths.tendermint, configTemplate, wd, p2pPort, websocketConfig)

          _ ← WorkerP2pConnectivity.make[F](app.id, producer.command[DialPeers[F]], app.cluster.workers)

          manifests ← WorkerBlockManifests.make[F](receiptStorage)

          _ ← blockUploading.start(
            app.id,
            manifests.receiptStorage,
            h ⇒ producer.blockStream(Some(h)),
            machine.command[ReceiptBus[F]],
            manifests.onUploaded
          )

          responder ← WorkerResponder.make(producer, machine)

          worker = Worker(
            app.id,
            machine,
            producer,
            machine.command[PeersControl[F]] :: responder :: HNil
          )

        } yield worker

    WorkersPool[F, Resources[F], Companions[F]] { (app, l) ⇒
      implicit val log: Log[F] = l
      WorkerContext(
        app,
        resources(app, ports, files),
        worker(app)
      )
    }
  }

}
