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
import fluence.kad.protocol.{Key, Node}
import fluence.kad.state.{Bucket, Siblings}

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
  ): LocalRouting[F, C] = new LocalRoutingImpl(nodeId, siblings, buckets)

}
