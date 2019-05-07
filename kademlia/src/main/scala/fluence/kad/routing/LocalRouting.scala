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

/**
 * LocalRouting describes how to route various requests over Kademlia network.
 * State is stored within [[Siblings]] and [[Bucket]], so there's no special case class.
 */
trait LocalRouting[F[_], C] {
  val nodeId: Key

  /**
   * Tries to route a key to a Contact, if it's known locally
   *
   * @param key Key to lookup
   */
  def find(key: Key): F[Option[Node[C]]]

  /**
   * Performs local lookup for the key, returning a stream of closest known nodes to it
   *
   * @param key Key to lookup
   * @return
   */
  def lookup(key: Key, numOfNodes: Int, predicate: Node[C] ⇒ Boolean = _ ⇒ true): F[Seq[Node[C]]]

  /**
   * Perform a lookup in local RoutingTable for a key,
   * return `numberOfNodes` closest known nodes, going away from the second key
   *
   * @param key Key to lookup
   * @param moveAwayFrom Key to move away
   */
  def lookupAway(key: Key, moveAwayFrom: Key, numOfNodes: Int): F[Seq[Node[C]]]

}

object LocalRouting {

  def apply[F[_]: Monad, P[_], C](nodeId: Key, siblings: F[Siblings[C]], buckets: Int ⇒ F[Bucket[C]])(
    implicit P: Parallel[F, P]
  ): LocalRouting[F, C] = new Impl(nodeId, siblings, buckets)

  private class Impl[F[_]: Monad, P[_], C](val nodeId: Key, siblings: F[Siblings[C]], buckets: Int ⇒ F[Bucket[C]])(
    implicit P: Parallel[F, P]
  ) extends LocalRouting[F, C] with slogging.LazyLogging {

    /**
     * Tries to route a key to a Contact, if it's known locally
     *
     * @param key Key to lookup
     */
    override def find(key: Key): F[Option[Node[C]]] =
      if (key === nodeId) Option.empty[Node[C]].pure[F]
      else
        P sequential P.apply.map2( // TODO: it's enough to find one non-empty reply; is there any way to explain it?
          P parallel siblings.map(_.find(key)),
          P parallel buckets((nodeId |+| key).zerosPrefixLen).map(_.find(key))
        )(_ orElse _)

    /**
     * Performs local lookup for the key, returning a stream of closest known nodes to it
     *
     * @param key Key to lookup
     * @return
     */
    override def lookup(key: Key, numOfNodes: Int, predicate: Node[C] ⇒ Boolean = _ ⇒ true): F[Seq[Node[C]]] = {

      implicit val ordering: Ordering[Node[C]] = Node.relativeOrdering(key)

      // Build stream of neighbors, taken from buckets
      val bucketsStream: Stream[F[Stream[Node[C]]]] = {
        // Base index: nodes as far from this one as the target key is
        val idx = (nodeId |+| key).zerosPrefixLen

        // Diverging stream of indices, going left (farther from current node) then right (closer), like 5 4 6 3 7 ...
        Stream(idx)
          .filter(_ < Key.BitLength) // In case current node is given, this will remove IndexOutOfBoundsException
          .append(
            Stream.from(1).takeWhile(i ⇒ idx + i < Key.BitLength || idx - i >= 0).flatMap { i ⇒
              (if (idx - i >= 0) Stream(idx - i) else Stream.empty) append
                (if (idx + i < Key.BitLength) Stream(idx + i) else Stream.empty)
            }
          )
          .map(
            idx ⇒
              // Take contacts from the bucket, and sort them
              buckets(idx).map(_.stream)
          )
      }

      // Fetch the minimal necessary amount of nodes close to target from buckets
      def fetchEnoughFromBuckets(
        more: Stream[F[Stream[Node[C]]]],
        collected: Queue[Node[C]] = Queue.empty
      ): F[Queue[Node[C]]] =
        more match {
          case _ if collected.length >= numOfNodes ⇒ // TODO: this is not optimal
            collected.pure[F]
          case next #:: evenMore ⇒
            next.map(_.filter(predicate)).flatMap { fetched ⇒
              fetchEnoughFromBuckets(evenMore, collected enqueue fetched)
            }
          case Stream.Empty ⇒
            collected.pure[F]
        }

      def combine(left: Stream[Node[C]], right: Stream[Node[C]], seen: Set[Key] = Set.empty): Stream[Node[C]] =
        (left, right) match {
          case (hl #:: tl, _) if seen(hl.key) ⇒ combine(tl, right, seen)
          case (_, hr #:: tr) if seen(hr.key) ⇒ combine(left, tr, seen)
          case (hl #:: tl, hr #:: _) if ordering.lt(hl, hr) ⇒ hl #:: combine(tl, right, seen + hl.key)
          case (hl #:: _, hr #:: tr) if ordering.gt(hl, hr) ⇒ hr #:: combine(left, tr, seen + hr.key)
          case (hl #:: tl, hr #:: tr) if ordering.equiv(hl, hr) ⇒ hr #:: combine(tl, tr, seen + hr.key)
          case (Stream.Empty, _) ⇒ right
          case (_, Stream.Empty) ⇒ left
        }

      P sequential P.apply.map2(
        // Stream of neighbors, taken from siblings
        P parallel siblings.map(_.nodes.filter(predicate).toStream.sorted),
        // Stream of buckets, sorted by closeness
        P parallel fetchEnoughFromBuckets(bucketsStream).map(_.toStream.sorted)
      )(
        // Combine stream, taking closer nodes first
        combine(_, _).take(numOfNodes)
      )
    }

    /**
     * Perform a lookup in local RoutingTable for a key,
     * return `numberOfNodes` closest known nodes, going away from the second key
     *
     * @param key          Key to lookup
     * @param moveAwayFrom Key to move away
     */
    override def lookupAway(key: Key, moveAwayFrom: Key, numOfNodes: Int): F[Seq[Node[C]]] =
      lookup(key, numOfNodes, n ⇒ (n.key |+| key) < (n.key |+| moveAwayFrom))

  }
}
