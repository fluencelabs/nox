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

import cats.{Applicative, Apply, Functor, Monad}
import cats.data.EitherT
import cats.syntax.applicative._
import cats.syntax.functor._
import fluence.effects.EffectError
import fluence.log.Log

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

  def destroy()(implicit log: Log[F]): EitherT[F, EffectError, Unit]
}

object WorkerResource {
  implicit def workerResourceFunctor[F[_]: Functor]: Functor[WorkerResource[F, *]] =
    new Functor[WorkerResource[F, *]] {
      override def map[A, B](fa: WorkerResource[F, A])(f: A ⇒ B): WorkerResource[F, B] =
        new WorkerResource[F, B] {
          override def prepare()(implicit log: Log[F]): F[B] = fa.prepare().map(f)

          override def destroy()(implicit log: Log[F]): EitherT[F, EffectError, Unit] = fa.destroy()
        }
    }

  implicit def workerResourceApplicative[F[_]: Monad]: Applicative[WorkerResource[F, *]] =
    new Applicative[WorkerResource[F, *]] {
      override def pure[A](x: A): WorkerResource[F, A] =
        new WorkerResource[F, A] {
          override def prepare()(implicit log: Log[F]): F[A] = x.pure[F]

          override def destroy()(implicit log: Log[F]): EitherT[F, EffectError, Unit] = EitherT.pure(())
        }

      override def ap[A, B](ff: WorkerResource[F, A ⇒ B])(fa: WorkerResource[F, A]): WorkerResource[F, B] =
        new WorkerResource[F, B] {
          override def prepare()(implicit log: Log[F]): F[B] =
            Applicative[F].ap(ff.prepare())(fa.prepare())

          override def destroy()(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
            Apply[EitherT[F, EffectError, *]].productR(ff.destroy())(fa.destroy())
        }
    }
}
