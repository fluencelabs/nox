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

package fluence.effects
import cats.Monad
import cats.data.EitherT
import cats.effect.Timer
import scala.language.higherKinds

package object syntax {

  object backoff {
    implicit class EitherTBackoffOps[F[_]: Timer: Monad, E <: EffectError: Backoff, T](fn: EitherT[F, E, T]) {
      def backoff: F[T] = implicitly[Backoff[E]].apply(fn)
    }
  }

  object eitherT {
    implicit class EitherTOps[F[_], A, B](ef: F[Either[A, B]]) {
      def eitherT: EitherT[F, A, B] = EitherT(ef)
    }
  }

}
