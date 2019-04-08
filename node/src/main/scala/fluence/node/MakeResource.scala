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
import cats.syntax.applicativeError._

import scala.language.higherKinds

object MakeResource extends LazyLogging {

  /**
   * Drains a stream concurrently while the resource is in use, interrupts it when it's released
   *
   * @param stream The stream to drain
   * @param name Stream's name, used for logging
   * @tparam F Effect
   * @return Resource that acts as a "stream is running" effect
   */
  def concurrentStream[F[_]: Concurrent](
    stream: fs2.Stream[F, _],
    name: String = "concurrentStream"
  ): Resource[F, Unit] =
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
          (exitCase match {
            case ExitCase.Error(e) ⇒
              logger.error(s"Stopping $name resource with error", e)
              stopDef.complete(Left(e)) *> streamFiber.join

            case ExitCase.Canceled ⇒
              logger.warn(s"Stopping $name resource due to Cancel")
              // Notice that we still .join the fiber
              stopDef.complete(Right(())) *> streamFiber.join

            case _ ⇒
              logger.debug(s"Stopping $name resource as it's not used anymore")
              stopDef.complete(Right(())) *> streamFiber.join
          }).map(_ ⇒ logger.debug(s"$name stopped, fiber joined"))
            .handleError(
              t ⇒
                // Do not raise errors during cleanup, catch them here to let other resources to clean them up
                logger.error(s"$name resource errored during stop: $t", t)
            )
      }
      .void

  /**
   * Simply lifts Ref.of to the Resource
   *
   * @param initial Initial value of Ref
   */
  def refOf[F[_]: Sync, T](initial: T): Resource[F, Ref[F, T]] =
    Resource.liftF(Ref.of(initial))

  /**
   * Order effects execution in the resource scope using a queue
   *
   * @tparam F Effect
   * @return (F[Unit] to execute) => F[Unit] to schedule, so that execution itself is non-blocking
   */
  def orderedEffects[F[_]: Concurrent]: Resource[F, F[Unit] ⇒ F[Unit]] =
    Resource
      .make(
        for {
          queue ← fs2.concurrent.Queue.noneTerminated[F, F[Unit]]
          fiber ← Concurrent[F].start(
            queue.dequeue.evalMap(identity).compile.drain
          )
        } yield (queue, fiber)
      ) {
        case (queue, fiber) ⇒
          // Terminate queue by submitting none, and wait until it stops
          queue.enqueue1(None) >> fiber.join
      }
      .map {
        case (queue, _) ⇒
          (fn: F[Unit]) ⇒
            queue.enqueue1(Some(fn))
      }

  def useConcurrently[F[_]: Concurrent](resource: F[Unit] ⇒ Resource[F, _]): F[Unit] =
    for {
      completeDef ← Deferred[F, Unit]
      fiberDef ← Deferred[F, Fiber[F, Unit]]
      fiber ← Concurrent[F].start(
        resource(
          completeDef.complete(()) >> fiberDef.get.flatMap(_.join)
        ).use(_ ⇒ completeDef.get)
      )
      _ ← fiberDef.complete(fiber)
    } yield ()
}
