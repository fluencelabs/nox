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

package fluence.kad.routing

import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.monoid._
import cats.syntax.order._
import cats.{Monad, Parallel}
import fluence.kad.protocol.{Key, Node}
import fluence.kad.state.{Bucket, Siblings}

import scala.collection.immutable.Queue
import scala.language.higherKinds

private[routing] class LocalRoutingImpl[F[_]: Monad, P[_], C](
  val nodeKey: Key,
  siblings: F[Siblings[C]],
  buckets: Int ⇒ F[Bucket[C]]
)(
  implicit P: Parallel[F, P]
) extends LocalRouting[F, C] with slogging.LazyLogging {

  override def find(key: Key): F[Option[Node[C]]] =
    if (key === nodeKey)
      Option.empty[Node[C]].pure[F]
    else
      Parallel.parAp2(
        ((_: Option[Node[C]]) orElse (_: Option[Node[C]])).pure[F]
      )(
        siblings.map(_.find(key)),
        buckets((nodeKey |+| key).zerosPrefixLen).map(_.find(key))
      )

  override def lookup(key: Key, neighbors: Int, predicate: Node[C] ⇒ Boolean = _ ⇒ true): F[Seq[Node[C]]] = {

    implicit val ordering: Ordering[Node[C]] = Node.relativeOrdering(key)

    // Build stream of neighbors, taken from buckets
    val bucketsStream: Stream[F[Stream[Node[C]]]] = {
      // Initial bucketId: nodes as far from this one as the target key is
      val bucketId = (nodeKey |+| key).zerosPrefixLen

      // Diverging stream of indices, going left (farther from current node) then right (closer), like 5 4 6 3 7 ...
      Stream(bucketId)
        .filter(_ < Key.BitLength) // In case current node is given, this will remove IndexOutOfBoundsException
        .append(
          Stream.from(1).takeWhile(i ⇒ bucketId + i < Key.BitLength || bucketId - i >= 0).flatMap { i ⇒
            (if (bucketId - i >= 0) Stream(bucketId - i) else Stream.empty) append
              (if (bucketId + i < Key.BitLength) Stream(bucketId + i) else Stream.empty)
          }
        )
        .map(
          bucketId ⇒
            // Take contacts from the bucket, and sort them
            buckets(bucketId).map(_.stream)
        )
    }

    /**
     * Fetch the minimal necessary amount of nodes close to target from buckets.
     * Later on these nodes will be mergeSorted with those taken from Siblings
     *
     * @param more see [[bucketsStream]]
     * @param collected Queue of collected nodes, used for recursion
     * @return
     */
    def fetchEnoughFromBuckets(
      more: Stream[F[Stream[Node[C]]]],
      collected: Queue[Node[C]] = Queue.empty
    ): F[Queue[Node[C]]] =
      more match {
        case _ if collected.length >= neighbors ⇒ // TODO: this is not optimal
          collected.pure[F]
        case next #:: evenMore ⇒
          next.map(_.filter(predicate)).flatMap { fetched ⇒
            fetchEnoughFromBuckets(evenMore, collected enqueue fetched)
          }
        case Stream.Empty ⇒
          collected.pure[F]
      }

    def mergeSorted(left: Stream[Node[C]], right: Stream[Node[C]], seen: Set[Key] = Set.empty): Stream[Node[C]] =
      (left, right) match {
        case (hl #:: tl, _) if seen(hl.key) ⇒ mergeSorted(tl, right, seen)
        case (_, hr #:: tr) if seen(hr.key) ⇒ mergeSorted(left, tr, seen)
        case (hl #:: tl, hr #:: _) if ordering.lt(hl, hr) ⇒ hl #:: mergeSorted(tl, right, seen + hl.key)
        case (hl #:: _, hr #:: tr) if ordering.gt(hl, hr) ⇒ hr #:: mergeSorted(left, tr, seen + hr.key)
        case (hl #:: tl, hr #:: tr) if ordering.equiv(hl, hr) ⇒ hr #:: mergeSorted(tl, tr, seen + hr.key)
        case (Stream.Empty, _) ⇒ right
        case (_, Stream.Empty) ⇒ left
      }

    type S = Stream[Node[C]]

    Parallel
      .parAp2((mergeSorted(_: S, _: S).take(neighbors)).pure[F])(
        // Sorted stream of neighbors, taken from siblings
        siblings.map(_.nodes.filter(predicate).toStream.sorted),
        // Sorted stream of neighbors, taken from buckets
        fetchEnoughFromBuckets(bucketsStream).map(_.toStream.sorted)
      )
      .map(_.toSeq)
  }
}
