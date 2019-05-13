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

package fluence.kad

import cats.data.EitherT
import cats.effect.{Clock, Effect}
import cats.Parallel
import fluence.kad.routing.{IterativeRouting, RoutingTable}
import fluence.kad.protocol.{ContactAccess, KademliaRpc, Key, Node}

import scala.language.higherKinds

trait Kademlia[F[_], C] {

  /**
   * Current node's Kademlia Key
   */
  val nodeKey: Key

  /**
   * Returns a network wrapper around a contact C, allowing querying it with Kademlia protocol
   *
   * @param contact Description on how to connect to remote node
   * @return
   */
  protected def rpc(contact: C): KademliaRpc[C]

  /**
   * How to promote this node to others
   */
  def ownContact: F[Node[C]]

  /**
   * Update RoutingTable with a freshly seen node
   *
   * @param node Discovered node, known to be alive and reachable
   * @return true if node is present in routing table after update, false if it was not added
   */
  def update(node: Node[C]): F[Boolean]

  /**
   * @return KademliaRPC instance to handle incoming RPC requests
   */
  def handleRPC: KademliaRpc[C]

  /**
   * Finds a node by its key, either in a local RoutingTable or doing up to ''maxRequests'' lookup calls
   *
   * @param key Kademlia key to find node for
   * @param maxRequests Max number of remote requests
   */
  def findNode(key: Key, maxRequests: Int): F[Option[Node[C]]]

  /**
   * Perform iterative lookup, see [[IterativeRouting.lookupIterative]]
   *
   * @param key Key to lookup
   * @param neighbors How many neighbors to return
   * @return key's neighborhood, .size <= neighbors
   */
  def lookupIterative(key: Key, neighbors: Int): F[Seq[Node[C]]]

  /**
   * Performs lookupIterative for a key, and then callIterative for neighborhood.
   * See [[IterativeRouting.callIterative]]
   *
   * @param key            Target key -- function will be called on this key's neighborhood nodes
   * @param fn             Function to call
   * @param takeFnResults   How many successful calls are expected to be made & results returned
   * @param maxFnCallTries  Max num of fn calls, both returning left and right side, before iterations are stopped
   * @param isIdempotentFn If true, there could be more then numToCollect successful calls made
   * @tparam A fn call type
   * @return Sequence of nodes with corresponding successful replies
   */
  def callIterative[E, A](
    key: Key,
    fn: Node[C] ⇒ EitherT[F, E, A],
    takeFnResults: Int,
    maxFnCallTries: Int,
    isIdempotentFn: Boolean = true
  ): F[Seq[(Node[C], A)]]

  /**
   * Joins the Kademlia network by a list of known peers
   *
   * @param peers Peers contact info
   * @return Whether joined or not
   */
  def join(peers: Seq[C], numberOfNodes: Int): F[Boolean]
}

object Kademlia {

  def apply[F[_]: Effect: Clock, P[_], C: ContactAccess](
    routing: RoutingTable[F, C],
    ownContactGetter: F[Node[C]],
    conf: KademliaConf
  )(implicit P: Parallel[F, P]): Kademlia[F, C] =
    new KademliaImpl(routing.nodeKey, conf.parallelism, ownContactGetter, routing)

}
