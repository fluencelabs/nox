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
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.applicativeError._
import fluence.log.Log

import scala.language.higherKinds

object MakeResource {

  /**
   * Drains a stream concurrently while the resource is in use, interrupts it when it's released
   *
   * @param stream The stream to drain
   * @param name Stream's name, used for logging
   * @tparam F Effect
   * @return Resource that acts as a "stream is running" effect
   */
  def concurrentStream[F[_]: Concurrent: Log](
    stream: fs2.Stream[F, _],
    name: String = "concurrentStream"
  ): Resource[F, Unit] =
    Resource
      .makeCase[F, (Deferred[F, Either[Throwable, Unit]], Fiber[F, Unit], Log[F])](
        for {
          stopDef ← Deferred[F, Either[Throwable, Unit]]
          streamFiber ← Concurrent[F].start(
            stream.interruptWhen(stopDef).compile.drain
          )
        } yield (stopDef, streamFiber, Log[F].getScoped(name))
      ) {
        case ((stopDef, streamFiber, log), exitCase) ⇒
          (exitCase match {
            case ExitCase.Error(e) ⇒
              log.error(s"Stopping with error", e) *>
                stopDef.complete(Left(e)) *> streamFiber.join

            case ExitCase.Canceled ⇒
              log.warn(s"Stopping due to Cancel") *>
                // Notice that we still .join the fiber
                stopDef.complete(Right(())) *> streamFiber.join

            case _ ⇒
              log.debug(s"Stopping as it's not used anymore") *>
                stopDef.complete(Right(())) *> streamFiber.join
          }).flatMap(_ ⇒ log.debug(s"Stopped, fiber joined"))
            .handleErrorWith(
              t ⇒
                // Do not raise errors during cleanup, catch them here to let other resources to clean them up
                log.error(s"Errored during stop: $t", t)
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
          (fn: F[Unit]) ⇒ queue.enqueue1(Some(fn))
      }

  /**
   * Uses the resource concurrently in a separate fiber, until the given `stop: F[Unit]` resolves.
   *
   * Example:
   * ```
   * // start some system, set `onStop` to be called when system is stopped internally (i.e. via API call)
   * useConcurrently(onStop => startSystem(onStop = onStop))
   * ```
   *
   * @param resource release (stop: F[Unit]) => resource
   * @tparam F Effect
   * @return Delayed action of using the resource
   */
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

  /**
   * Allocates resource in (possible concurrent) context, defined by `evalOn` function
   * Use this when you need to go from Resource[F, T] to Resource[ F, F[T] ] (especially concurrently)
   *
   * @param resource Resource to be allocated
   * @param evalOn Description of how to evaluate resource
   * @return Resource that holds promise of result instead of result itself
   */
  def allocateOn[F[_]: Concurrent, T](resource: Resource[F, T], evalOn: F[Unit] => F[Unit]): Resource[F, F[T]] =
    for {
      deferred <- Resource.liftF(Deferred[F, (T, F[Unit])])
      // Promise with result
      resultF = deferred.get.map(_._1)
      // Will stop resource on evaluation
      stop = deferred.get.flatMap(_._2)
      // Allocate resource, evaluating it via `evalOn`; stop resource when it is not needed anymore
      _ <- Resource.make(evalOn(resource.allocated >>= deferred.complete))(_ => stop)
    } yield resultF
}
