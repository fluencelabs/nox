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

import cats.syntax.applicative._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.order._
import cats.{FlatMap, Monad, Order}

import scala.language.higherKinds

object QueueSyntax {
  implicit class RichQueue[F[_]: Monad, A: HasOrderedProperty[*, T], T: Order](queue: fs2.concurrent.Queue[F, A]) {
    import HasOrderedProperty.syntax._

    /**
     * Dequeues queue until element with specified boundary is found. All elements with lower boundary are dropped.
     *
     * Asynchronously blocks until element is found. If queue elements are out of order, sorts them by requeuing.
     *
     * @param boundary Target boundary
     * @return Element with specified boundary
     */
    def dequeueByBoundary(boundary: T): F[A] =
      FlatMap[F].tailRecM(queue) { q =>
        q.dequeue1.flatMap { elem =>
          if (elem.key < boundary) {
            // keep looking
            q.asLeft[A].pure[F]
          } else if (elem.key > boundary) {
            // corner case: elements aren't in order, try to reorder them
            q.enqueue1(elem).as(q.asLeft)
          } else {
            // got it!
            elem.asRight[fs2.concurrent.Queue[F, A]].pure[F]
          }
        }
      }
  }

}
