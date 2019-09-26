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

package fluence.worker

import cats.Monad
import cats.data.{EitherT, OptionT}
import cats.effect.Sync
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.effect.concurrent.Ref
import fluence.log.Log
import fluence.worker.eth.EthApp

import scala.language.higherKinds

class WorkersPool[F[_]: Monad, R, C](
  workers: Ref[F, Map[Long, WorkerContext[F, R, C]]],
  appWorker: (EthApp, Log[F]) ⇒ F[WorkerContext[F, R, C]]
) {

  /**
   * Run a new worker with the given EthApp
   *
   * @param app Application description how it comes from Ethereum
   * @return Current WorkerStage
   */
  def run(app: EthApp)(implicit log: Log[F]): F[WorkerStage] =
    // TODO as we have no mutex here, it is possible to run worker twice
    get(app.id)
      .getOrElseF(
        appWorker(app, log) >>= (w ⇒ workers.update(_ + (app.id -> w)).as(w))
        // TODO: when worker is destroyed, it should be eventually removed from the cache
        // to do so, we could either subscribe concurrently to `stages` and wait for Destroyed stage,
        // or modify WorkerContext to take a onDestroyed callback (but then onStopped should also provided, and maybe more...)
      )
      .flatMap(_.stage)

  /**
   * Get a worker context, if it was launched
   *
   * @param appId Application id
   * @return
   */
  def get(appId: Long): OptionT[F, WorkerContext[F, R, C]] =
    OptionT(workers.get.map(_.get(appId)))

  /**
   * Get worker resources, if worker is known
   */
  def getResources(appId: Long): OptionT[F, R] =
    get(appId).map(_.resources)

  /**
   * Get worker, if it's known and launched
   */
  def getWorker(appId: Long): EitherT[F, WorkerStage, Worker[F]] =
    get(appId).toRight[WorkerStage](WorkerStage.NotInitialized).flatMap(_.worker)

  /**
   * Get worker companions, if known and launched
   */
  def getCompanions(appId: Long): EitherT[F, WorkerStage, C] =
    get(appId).toRight[WorkerStage](WorkerStage.NotInitialized).flatMap(_.companions)

  /**
   * List all launched workers
   */
  def listAll(): F[List[WorkerContext[F, R, C]]] =
    workers.get.map(_.values.toList)

}

object WorkersPool {

  def apply[F[_]: Sync, R, C](
    appWorkerCtx: (EthApp, Log[F]) ⇒ F[WorkerContext[F, R, C]]
  ): F[WorkersPool[F, R, C]] =
    Ref
      .of[F, Map[Long, WorkerContext[F, R, C]]](Map.empty)
      .map(
        new WorkersPool[F, R, C](_, appWorkerCtx)
      )

}
