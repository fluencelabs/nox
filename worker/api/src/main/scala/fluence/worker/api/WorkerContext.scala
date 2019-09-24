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

package fluence.worker.api

import cats.syntax.flatMap._
import cats.syntax.apply._
import cats.syntax.functor._
import cats.effect.{Concurrent, Resource}
import cats.effect.concurrent.Deferred
import fluence.effects.resources.MakeResource
import fluence.log.Log

import scala.language.higherKinds

trait WorkerContext[F[_]] {
  def appId: Long

  //def app: App

  type Resources

  def resources: Resources

  type W <: Worker[F]

  type Companions

  def worker: F[W]

  def companions: F[Companions]

  def stop()(implicit log: Log[F]): F[Unit]

  def remove()(implicit log: Log[F]): F[Unit]
}

object WorkerContext {
  type Aux[F[_], R, W0 <: Worker[F], C] = WorkerContext[F]{
  type Resources = R
  type Companions = C
  type W = W0
  }

  def prepare[F[_]: Concurrent: Log, R, W0 <: Worker[F], C](
    _appId: Long,
    // app: App,
    workerResource: WorkerResource[F, R],
    worker: R ⇒ Resource[F, W0],
    companions: WorkerCompanion.Aux[F, C, W0]
                                         ): F[WorkerContext.Aux[F, R, W0, C]] =
    workerResource.prepare() >>= {res ⇒
      for {
        deferred ← Deferred[F, (W0, C)]
        stopDef ← Deferred[F, F[Unit]]
      _ ← MakeResource.useConcurrently[F](stop ⇒
      // TODO here we have a lot of lifecycle information, reflect it!
        worker(res) >>= (w ⇒ companions.resource(w).map(w -> _) >>= (wx ⇒ Resource.liftF(stopDef.complete(stop) *> deferred.complete(wx))))
      )
      } yield new WorkerContext[F] {
        override def appId: Long = _appId

        override type Resources = R

        override val resources: Resources = res

        override type W = W0
        override type Companions = C

        // TODO return eithert, show lifecycle on the left (preparing, acquiring companions, acquired (fetch worker status), stopped (enable restart), removed
        // TODO it would be nice to run worker lazily, on first start
        override def worker: F[W] = deferred.get.map(_._1)
deferred.complete()
        // TODO return eithert, show lifecycle on the left
        override def companions: F[C] = deferred.get.map(_._2)

        // TODO should not be able to get worker or companions any more
        override def stop()(implicit log: Log[F]): F[Unit] = stopDef.get.flatten

        // TODO should not be able to get resources as well
        override def remove()(implicit log: Log[F]): F[Unit] = stopDef.get.flatten >> workerResource.remove().value.void
      }

    }
}
