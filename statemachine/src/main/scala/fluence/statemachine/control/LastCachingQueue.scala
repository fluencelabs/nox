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

package fluence.statemachine.control

import cats.effect.Concurrent
import cats.effect.concurrent.Ref
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Monad, Order}
import fluence.statemachine.control.HasOrderedProperty.syntax._

import scala.language.higherKinds

/**
 * Concurrent Queue of ordered elements allowing to retrieve element with specific position.
 * Caches last element, so it can be retrieved several times.
 *
 * Implemented as a wrapper around fs2.concurrent.Queue and cats.effect.concurrent.Ref
 *
 * @param queue Queue with elements
 * @param ref Ref to hold last element in case of retry
 */
class LastCachingQueue[F[_]: Monad, A: HasOrderedProperty[?, T], T: Order](
  queue: fs2.concurrent.Queue[F, A],
  ref: Ref[F, Option[A]]
) {

  def enqueue1(a: A): F[Unit] = queue.enqueue1(a)

  /**
   * Dequeues queue until element with specified boundary is found. All elements with lower boundary are dropped.
   *
   * Asynchronously blocks until element is found. If queue elements are out of order, sorts them by requeuing.
   *
   * @param boundary Target boundary
   * @return Element with specified boundary
   */
  def dequeue(boundary: T): F[A] =
    ref.get.flatMap {
      case Some(elem) if elem.key == boundary => elem.pure[F]
      case _                                  => queue.dequeueByBoundary(boundary).flatTap(cache)
    }

  private def cache(a: A) = ref.set(Some(a))
}

object LastCachingQueue {

  def apply[F[_]: Concurrent, A: HasOrderedProperty[?, T], T: Order]: F[LastCachingQueue[F, A, T]] =
    for {
      queue <- fs2.concurrent.Queue.unbounded[F, A]
      ref <- Ref.of[F, Option[A]](None)
    } yield new LastCachingQueue(queue, ref)
}
