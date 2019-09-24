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

import cats.{Applicative, Parallel}
import cats.data.EitherT
import cats.syntax.parallel._
import cats.syntax.apply._
import cats.syntax.applicative._
import cats.instances.either._
import fluence.effects.EffectError
import fluence.log.Log
import shapeless._

import scala.language.higherKinds

/**
 * WorkerResource is some part of Worker context that gets prepared before Worker can be launched,
 * and that lasts after it is stopped.
 * For example, it could be WASM packages, Tendermint config files, p2p port, folders, etc.
 * Worker can use Resources to be built.
 *
 * @tparam F Effect
 * @tparam T Resource type. Note: resource should be provided ASAP, with no fiber blocking. If you need, run a concurrent process.
 */
trait WorkerResource[F[_], T] {

  def prepare()(implicit log: Log[F]): F[T]

  def remove()(implicit log: Log[F]): EitherT[F, EffectError, Unit]

}

object WorkerResource {
  abstract class Extensible[F[_]: Applicative: Parallel, R <: HList] extends WorkerResource[F, R] {
    self ⇒

    def extend[T](resource: WorkerResource[F, T]): Extensible[F, T :: R] = new Extensible[F, T :: R] {
      override def prepare()(implicit log: Log[F]): F[T :: R] =
        (resource.prepare(), self.prepare()).parMapN(_ :: _)

      override def remove()(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
        EitherT((self.remove().value, resource.remove().value).parMapN((a, b) ⇒ a *> b ))
    }
  }

  def empty[F[_]: Applicative: Parallel]: WorkerResource[F, HNil] = new Extensible[F, HNil] {
    override def prepare()(implicit log: Log[F]): F[HNil] = (HNil: HNil).pure[F]

    override def remove()(implicit log: Log[F]): EitherT[F, EffectError, Unit] = EitherT.rightT(())
  }
}
