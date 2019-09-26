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

import cats.Parallel
import cats.effect.{Concurrent, ContextShift, LiftIO, Resource, Sync, Timer}
import fluence.bp.api.DialPeers
import fluence.bp.uploading.BlockUploading
import fluence.effects.{Backoff, EffectError}
import fluence.effects.docker.DockerIO
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.block.data.Block
import fluence.log.Log
import fluence.node.workers.WorkerBlockManifests
import fluence.node.workers.pool.{WorkerP2pConnectivity, WorkersPorts}
import fluence.statemachine.api.command.ReceiptBus
import fluence.worker.{Worker, WorkerContext, WorkerResource, WorkersPool}
import fluence.worker.eth.EthApp
import fluence.worker.responder.WorkerResponder
import shapeless._

import scala.language.higherKinds

object MasterPool {

  type Resources[F[_]] = WorkersPorts.P2pPort[F]
  type Companions[F[_]] = WorkerResponder[F]

  def resources[F[_]](
    app: EthApp,
    ports: WorkersPorts[F]
  ): WorkerResource[F, Resources[F]] = {
    ports.workerResource(app.id)
  }

  def companions[F[_]: Sync: LiftIO: ContextShift: Timer: Concurrent: SttpEffect: Parallel, PC <: HList, MC <: HList, W <: Worker.Aux[
    F,
    MC,
    Block,
    PC
  ]](
    app: EthApp,
    receiptStorage: Resource[F, ReceiptStorage[F]],
    blockUploading: BlockUploading[F]
  )(
    implicit
    dp: ops.hlist.Selector[PC, DialPeers[F]],
    rb: ops.hlist.Selector[MC, ReceiptBus[F]],
    backoff: Backoff[EffectError]
  ): (W, Log[F]) ⇒ Resource[F, Companions[F]] = { (worker, l) ⇒
    implicit val log: Log[F] = l

    for {
      _ ← WorkerP2pConnectivity.make[F](app.id, worker.producer.command[DialPeers[F]], app.cluster.workers)

      manifests ← WorkerBlockManifests.make[F](receiptStorage)

      _ ← blockUploading.start(
        app.id,
        manifests.receiptStorage,
        h ⇒ worker.producer.blockStream(Some(h)),
        worker.machine.command[ReceiptBus[F]],
        manifests.onUploaded
      )

      responder ← WorkerResponder.make(worker)
    } yield responder

  }

  def apply[F[_]: Parallel: Timer: Concurrent: DockerIO: LiftIO: ContextShift](
    ports: WorkersPorts[F],
    // TODO make them companions/resources?
    receiptStorage: Resource[F, ReceiptStorage[F]],
    blockUploading: BlockUploading[F]
  )(implicit backoff: Backoff[EffectError], sttp: SttpEffect[F]): F[WorkersPool[F, Resources[F], Companions[F]]] = {

    type ProducerComponents = DialPeers[F] :: HNil
    type MachineComponents = ReceiptBus[F] :: HNil

    type W0 = Worker.Aux[F, MachineComponents, Block, ProducerComponents]

    def worker(app: EthApp)(implicit log: Log[F]): Resources[F] ⇒ Resource[F, W0] = ???

    def context(app: EthApp)(implicit log: Log[F]): F[WorkerContext[F, Resources[F], Companions[F]]] =
      WorkerContext(
        app,
        resources(app, ports),
        worker(app),
        companions[F, ProducerComponents, MachineComponents, W0](app, receiptStorage, blockUploading)
      )

    WorkersPool(
      (e, l) ⇒ context(e)(l)
    )
  }

}
