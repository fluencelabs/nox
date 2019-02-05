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
import cats.effect._
import cats.effect.concurrent.{Deferred, Ref}
import slogging.LazyLogging
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.flatMap._

import scala.language.higherKinds

object MakeResource extends LazyLogging {

  /**
   * Drains a stream concurrently while the resource is in use, interrupts it when it's released
   *
   * @param stream The stream to drain
   * @tparam F Effect
   * @return Resource that acts as a "stream is running" effect
   */
  def concurrentStream[F[_]: Concurrent](stream: fs2.Stream[F, _]): Resource[F, Unit] =
    Resource
      .makeCase[F, (Deferred[F, Either[Throwable, Unit]], Fiber[F, Unit])](
        for {
          stopDef ← Deferred[F, Either[Throwable, Unit]]
          streamFiber ← Concurrent[F].start(
            stream.interruptWhen(stopDef).compile.drain
          )
        } yield stopDef → streamFiber
      ) {
        case ((stopDef, streamFiber), exitCase) ⇒
          exitCase match {
            case ExitCase.Error(e) ⇒
              logger.error("concurrentStream resource is closed with error", e)
              stopDef.complete(Left(e)) *> streamFiber.join

            case ExitCase.Canceled ⇒
              logger.warn("concurrentStream resource is closed due to Cancel")
              stopDef.complete(Right(())) *> streamFiber.cancel

            case _ ⇒
              logger.debug("concurrentStream resource is closed as it's not used any more")
              stopDef.complete(Right(())) *> streamFiber.join
          }
      }
      .void

  /**
   * Simply lifts Ref.of to the Resource
   *
   * @param initial Initial value of Ref
   */
  def refOf[F[_]: Sync, T](initial: T): Resource[F, Ref[F, T]] =
    Resource.liftF(Ref.of(initial))
}
