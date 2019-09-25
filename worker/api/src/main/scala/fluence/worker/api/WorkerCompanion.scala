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

import cats.Monad
import cats.effect.Resource
import fluence.log.Log

import scala.language.higherKinds

/**
 * WorkerCompanion represents resources which lifetime is inside the Worker's lifetime:
 * they are allocated when Worker is already available, and deallocated before Worker is.
 *
 * @tparam F Effect
 * @tparam T Companion type
 */
abstract class WorkerCompanion[F[_]: Monad, T] {
  self ⇒
  type W <: Worker[F]

  def resource(worker: W)(implicit log: Log[F]): Resource[F, T]

  final def map[TT](fn: T ⇒ TT): WorkerCompanion.Aux[F, TT, W] = new WorkerCompanion[F, TT] {
    override type W = self.W

    override def resource(worker: W)(implicit log: Log[F]): Resource[F, TT] =
      self.resource(worker).map(fn)
  }

  final def flatMap[TT](other: T ⇒ WorkerCompanion.Aux[F, TT, W]): WorkerCompanion.Aux[F, TT, W] =
    new WorkerCompanion[F, TT] {
      override type W = self.W

      override def resource(worker: W)(implicit log: Log[F]): Resource[F, TT] =
        for {
          selfResource ← self.resource(worker)
          res ← other(selfResource).resource(worker)
        } yield res
    }
}

object WorkerCompanion {
  type Aux[F[_], T, W0 <: Worker[F]] = WorkerCompanion[F, T] { type W = W0 }
}
