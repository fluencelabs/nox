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

import cats.Applicative
import cats.effect.{Concurrent, Resource, Timer}
import cats.effect.concurrent.{MVar, Ref}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.log.Log
import fluence.node.workers.{Worker, WorkerParams, WorkerServices, WorkersPool}
import fluence.node.workers.subscription.ResponseSubscriber

import scala.language.higherKinds

class CustomWorkersPool[F[_]: Concurrent](
  workers: MVar[F, Map[Long, Worker[F]]],
  servicesBuilder: Long => WorkerServices[F]
) extends WorkersPool[F] {

  /**
   * Run or restart a worker
   *
   * @param params Worker's description
   * @return Whether worker run or not
   */
  override def run(appId: Long, params: F[WorkerParams])(implicit log: Log[F]): F[WorkersPool.RunResult] =
    workers.take.flatMap {
      case m if m.contains(appId) ⇒ workers.put(m).as(WorkersPool.AlreadyRunning)
      case m ⇒
        for {
          p ← params
          w ← Worker
            .make[F](
              appId,
              0: Short,
              s"Test worker for appId $appId",
              servicesBuilder(appId),
              identity,
              workers.take.flatMap(ws => workers.put(ws - appId)),
              Applicative[F].unit
            )
            .allocated
            .map(_._1)
          _ ← workers.put(m + (appId -> w))
        } yield WorkersPool.Starting
    }

  /**
   * Get a Worker by its appId, if it's present
   *
   * @param appId Application id
   * @return Worker
   */
  override def get(appId: Long): F[Option[Worker[F]]] =
    workers.read.map(_.get(appId))

  /**
   * Get all known workers
   *
   * @return Up-to-date list of workers
   */
  override def getAll: F[List[Worker[F]]] =
    workers.read.map(_.values.toList)
}

object CustomWorkersPool {

  def withRequestResponder[F[_]: Concurrent: Timer](
    requestResponder: ResponseSubscriber[F],
    tendermintRpc: TendermintRpc[F]
  ): F[CustomWorkersPool[F]] = {
    val builder = TestWorkerServices.workerServiceTestRequestResponse[F](tendermintRpc, requestResponder) _
    MVar.of(Map.empty[Long, Worker[F]]).map(new CustomWorkersPool(_, builder))
  }
}
