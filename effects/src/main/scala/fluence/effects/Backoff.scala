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

import cats.{Applicative, Monad}
import cats.data.EitherT
import cats.effect.Timer
import cats.syntax.flatMap._
import cats.syntax.apply._

import scala.concurrent.duration._
import scala.language.higherKinds

/**
 * Exponential backoff delays.
 *
 * @param delayPeriod will be applied next time
 * @param maxDelay upper bound for a single delay
 */
case class Backoff[E <: EffectError](delayPeriod: FiniteDuration, maxDelay: FiniteDuration) {

  /**
   * Next retry policy with delayPeriod multiplied times two, if maxDelay is not yet reached
   */
  def next: Backoff[E] =
    if (delayPeriod == maxDelay) this
    else {
      val nextDelay = delayPeriod * 2
      if (nextDelay > maxDelay) copy(delayPeriod = maxDelay) else copy(delayPeriod = nextDelay)
    }

  def retry[F[_]: Timer: Monad, EE <: E, T](fn: EitherT[F, EE, T], onError: EE ⇒ F[Unit]): F[T] =
    fn.value.flatMap {
      case Right(value) ⇒ Applicative[F].pure(value)
      case Left(err) ⇒
        onError(err) *> Timer[F].sleep(delayPeriod) *> next.retry(fn, onError)
    }

  def apply[F[_]: Timer: Monad, EE <: E, T](fn: EitherT[F, EE, T]): F[T] =
    retry(fn, (_: EE) ⇒ Applicative[F].unit)

}

object Backoff {
  def default[E <: EffectError]: Backoff[E] = Backoff(1.second, 1.minute)
}
