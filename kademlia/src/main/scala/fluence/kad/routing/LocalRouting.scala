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

import cats.{Monad, Parallel}
import cats.syntax.monoid._
import cats.syntax.order._
import fluence.kad.protocol.{Key, Node}
import fluence.kad.state.{Bucket, Siblings}

import scala.language.higherKinds

/**
 * LocalRouting provides Kademlia routing using locally stored routing data only.
 * State is stored within [[Siblings]] and [[Bucket]] case classes.
 */
trait LocalRouting[F[_], C] {
  val nodeKey: Key

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
   * @param neighbors Number of key's neighbors to return
   * @param predicate Predicate to test nodes against prior to returning
   * @return Sequence of nodes, ordered by distance to the given key
   */
  def lookup(key: Key, neighbors: Int, predicate: Node[C] ⇒ Boolean = _ ⇒ true): F[Seq[Node[C]]]

  /**
   * Perform a lookup in local RoutingTable for a key, skipping nodes closer to moveAwayFrom than to key
   *
   * @param key Key to lookup
   * @param moveAwayFrom Key to move away from
   * @param neighbors Number of key's neighbors to return
   */
  def lookupAway(key: Key, moveAwayFrom: Key, neighbors: Int): F[Seq[Node[C]]] =
    lookup(key, neighbors, n ⇒ (n.key |+| key) < (n.key |+| moveAwayFrom))

}

object LocalRouting {

  def apply[F[_]: Monad, P[_], C](nodeKey: Key, siblings: F[Siblings[C]], buckets: Int ⇒ F[Bucket[C]])(
    implicit P: Parallel[F, P]
  ): LocalRouting[F, C] = new LocalRoutingImpl(nodeKey, siblings, buckets)

}
