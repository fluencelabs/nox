/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.kad

import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.eq._
import cats.{ MonadError, Parallel }
import fluence.kad.RoutingTable._
import fluence.kad.protocol.{ KademliaRpc, Key, Node }
import slogging.LazyLogging

import scala.concurrent.duration.Duration
import scala.language.higherKinds
import scala.util.control.NoStackTrace

/**
 * Kademlia interface for current node and all Kademlia-related RPC calls, both incoming and outgoing
 *
 * @param nodeId Current node's Kademlia key
 * @param parallelism   Parallelism factor (named Alpha in paper)
 * @param pingExpiresIn Duration to avoid too frequent ping requests, used in [[Bucket.update()]]
 * @param checkNode Check node correctness, e.g. signatures are correct, ip is public, etc.
 * @param F            Monad error
 * @tparam F Effect
 * @tparam C Contact info
 */
abstract class Kademlia[F[_], C](
    val nodeId: Key,
    parallelism: Int,
    val pingExpiresIn: Duration,
    checkNode: Node[C] ⇒ F[Boolean]
)(implicit F: MonadError[F, Throwable], P: Parallel[F, F], BW: Bucket.WriteOps[F, C], SW: Siblings.WriteOps[F, C]) {
  self ⇒

  /**
   * Returns a network wrapper around a contact C, allowing querying it with Kademlia protocol
   *
   * @param contact Description on how to connect to remote node
   * @return
   */
  def rpc(contact: C): KademliaRpc[F, C]

  /**
   * How to promote this node to others
   */
  def ownContact: F[Node[C]]

  /**
   * Update RoutingTable with a freshly seen node
   *
   * @param node Discovered node, known to be alive and reachable
   * @return true if node is present in routing table after update, false if it's dropped
   */
  def update(node: Node[C]): F[Boolean] =
    nodeId.update(node, rpc, pingExpiresIn, checkNode)

  /**
   * @return KademliaRPC instance to handle incoming RPC requests
   */
  val handleRPC: KademliaRpc[F, C] = new KademliaRpc[F, C] with LazyLogging {

    /**
     * Respond for a ping with node's own contact data
     *
     * @return
     */
    override def ping(): F[Node[C]] = {
      logger.trace(s"HandleRPC($nodeId): ping")
      ownContact
    }

    /**
     * Perform a lookup in local RoutingTable
     *
     * @param key Key to lookup
     * @param numberOfNodes How many nodes to return (upper bound)
     * @return locally known neighborhood
     */
    override def lookup(key: Key, numberOfNodes: Int): F[Seq[Node[C]]] = {
      logger.trace(s"HandleRPC($nodeId): lookup($key, $numberOfNodes)")
    }.pure[F].map(_ ⇒ nodeId.lookup(key).take(numberOfNodes))

    /**
     * Perform a lookup in local RoutingTable for a key,
     * return `numberOfNodes` closest known nodes, going away from the second key
     *
     * @param key Key to lookup
     * @param numberOfNodes How many nodes to return (upper bound)
     */
    override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): F[Seq[Node[C]]] = {
      logger.trace(s"HandleRPC($nodeId): lookupAway($key, $moveAwayFrom, $numberOfNodes)")
    }.pure[F]
      .map(
        _ ⇒
          nodeId
            .lookupAway(key, moveAwayFrom)
            .take(numberOfNodes)
      )
  }

  /**
   * Finds a node by its key, either in a local RoutingTable or doing up to ''maxRequests'' lookup calls
   *
   * @param key Kademlia key to find node for
   * @param maxRequests Max number of remote requests
   */
  def findNode(key: Key, maxRequests: Int): F[Option[Node[C]]] =
    nodeId.find(key) match {
      case found @ Some(_) ⇒ (found: Option[Node[C]]).pure[F]

      case None ⇒
        callIterative(
          key,
          n ⇒
            if (n.key === key) F.pure(())
            else F.raiseError[Unit](new RuntimeException("Mismatching node") with NoStackTrace),
          numToCollect = 1,
          maxNumOfCalls = maxRequests
        ).map(_.headOption.map(_._1))
    }

  /**
   * Perform iterative lookup, see [[RoutingTable.WriteOps.lookupIterative]]
   *
   * @param key Key to lookup
   * @return key's neighborhood
   */
  def lookupIterative(key: Key, numberOfNodes: Int): F[Seq[Node[C]]] =
    nodeId.lookupIterative(key, numberOfNodes, parallelism, rpc, pingExpiresIn, checkNode)

  /**
   * Performs lookupIterative for a key, and then callIterative for neighborhood.
   * See [[RoutingTable.WriteOps.callIterative]]
   *
   * @param key            Key to call function near
   * @param fn             Function to call
   * @param numToCollect   How many calls are expected to be made
   * @param maxNumOfCalls  Max num of calls before iterations are stopped
   * @param isIdempotentFn If true, there could be more then numToCollect successful calls made
   * @tparam A fn call type
   * @return Sequence of nodes with corresponding successful replies, should be >= numToCollect in case of success
   */
  def callIterative[A](
      key: Key,
      fn: Node[C] ⇒ F[A],
      numToCollect: Int,
      maxNumOfCalls: Int,
      isIdempotentFn: Boolean = true
  ): F[Seq[(Node[C], A)]] =
    nodeId.callIterative(key,
                         fn,
                         numToCollect,
                         parallelism,
                         maxNumOfCalls,
                         isIdempotentFn,
                         rpc,
                         pingExpiresIn,
                         checkNode)

  /**
   * Joins the Kademlia network by a list of known peers. Fails if no join operations performed successfully
   *
   * @param peers Peers contact info
   * @return
   */
  def join(peers: Seq[C], numberOfNodes: Int): F[Unit] =
    nodeId.join(peers, rpc, pingExpiresIn, numberOfNodes, checkNode, parallelism)
}
