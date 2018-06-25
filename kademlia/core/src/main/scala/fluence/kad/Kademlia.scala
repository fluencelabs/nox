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

import cats.data.EitherT
import cats.effect.{IO, LiftIO, Timer}
import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.eq._
import cats.{Monad, Parallel}
import fluence.kad.protocol.{KademliaRpc, Key, Node}
import slogging.LazyLogging

import scala.concurrent.duration.Duration
import scala.language.{existentials, higherKinds}

trait Kademlia[F[_], C] {

  /**
   * Current node's Kademlia Key
   */
  val nodeId: Key

  /**
   * Returns a network wrapper around a contact C, allowing querying it with Kademlia protocol
   *
   * @param contact Description on how to connect to remote node
   * @return
   */
  def rpc(contact: C): KademliaRpc[C]

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
   * Perform iterative lookup, see [[RoutingTable.lookupIterative]]
   *
   * @param key Key to lookup
   * @return key's neighborhood
   */
  def lookupIterative(key: Key, numberOfNodes: Int): F[Seq[Node[C]]]

  /**
   * Performs lookupIterative for a key, and then callIterative for neighborhood.
   * See [[RoutingTable.callIterative]]
   *
   * @param key            Key to call function near
   * @param fn             Function to call
   * @param numToCollect   How many calls are expected to be made
   * @param maxNumOfCalls  Max num of calls before iterations are stopped
   * @param isIdempotentFn If true, there could be more then numToCollect successful calls made
   * @tparam A fn call type
   * @return Sequence of nodes with corresponding successful replies, should be >= numToCollect in case of success
   */
  def callIterative[E, A](
    key: Key,
    fn: Node[C] ⇒ EitherT[F, E, A],
    numToCollect: Int,
    maxNumOfCalls: Int,
    isIdempotentFn: Boolean = true
  ): F[Seq[(Node[C], A)]]

  /**
   * Joins the Kademlia network by a list of known peers. Fails if no join operations performed successfully
   *
   * @param peers Peers contact info
   * @return Whether joined or not
   */
  def join(peers: Seq[C], numberOfNodes: Int): F[Boolean]
}

object Kademlia {

  /**
   * Kademlia interface for current node and all Kademlia-related RPC calls, both incoming and outgoing
   *
   * @param nodeIdK        Current node's Kademlia key
   * @param parallelism   Parallelism factor (named Alpha in paper)
   * @param pingExpiresIn Duration to avoid too frequent ping requests, used in [[Bucket.update()]]
   * @param checkNode     Check node correctness, e.g. signatures are correct, ip is public, etc.
   * @param F             Monad error
   * @tparam F Effect
   * @tparam C Contact info
   */
  def apply[F[_], G[_], C](
    nodeIdK: Key,
    parallelism: Int,
    pingExpiresIn: Duration,
    checkNode: Node[C] ⇒ IO[Boolean],
    ownContactGetter: IO[Node[C]],
    kademliaRpc: C ⇒ KademliaRpc[C]
  )(
    implicit F: Monad[F],
    LI: LiftIO[F],
    T: Timer[F],
    P: Parallel[F, G],
    BW: Bucket.WriteOps[F, C],
    SW: Siblings.WriteOps[F, C]
  ): Kademlia[F, C] = new Kademlia[F, C] {
    self ⇒

    val routingTable = new RoutingTable[C](nodeIdK)

    override val nodeId: Key = nodeIdK

    /**
     * Returns a network wrapper around a contact C, allowing querying it with Kademlia protocol
     *
     * @param contact Description on how to connect to remote node
     * @return
     */
    override def rpc(contact: C): KademliaRpc[C] = kademliaRpc(contact)

    /**
     * How to promote this node to others
     */
    override val ownContact: F[Node[C]] = ownContactGetter.to[F]

    /**
     * Update RoutingTable with a freshly seen node
     * TODO: return the reason, why its not added to the routing table
     *
     * @param node Discovered node, known to be alive and reachable
     * @return true if node is present in routing table after update, false if it's dropped
     */
    override def update(node: Node[C]): F[Boolean] =
      routingTable.update(node, rpc, pingExpiresIn, checkNode)

    /**
     * @return KademliaRPC instance to handle incoming RPC requests
     */
    override val handleRPC: KademliaRpc[C] = new KademliaRpc[C] with LazyLogging {

      /**
       * Respond for a ping with node's own contact data
       *
       * @return
       */
      override def ping(): IO[Node[C]] = {
        logger.trace(s"HandleRPC($nodeId): ping")
        ownContactGetter
      }

      /**
       * Perform a lookup in local RoutingTable
       *
       * @param key           Key to lookup
       * @param numberOfNodes How many nodes to return (upper bound)
       * @return locally known neighborhood
       */
      override def lookup(key: Key, numberOfNodes: Int): IO[Seq[Node[C]]] =
        IO {
          logger.trace(s"HandleRPC($nodeId): lookup($key, $numberOfNodes)")

          routingTable.lookup(key).take(numberOfNodes)
        }

      /**
       * Perform a lookup in local RoutingTable for a key,
       * return `numberOfNodes` closest known nodes, going away from the second key
       *
       * @param key           Key to lookup
       * @param numberOfNodes How many nodes to return (upper bound)
       */
      override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): IO[Seq[Node[C]]] =
        IO {
          logger.trace(s"HandleRPC($nodeId): lookupAway($key, $moveAwayFrom, $numberOfNodes)")

          routingTable
            .lookupAway(key, moveAwayFrom)
            .take(numberOfNodes)
        }
    }

    /**
     * Finds a node by its key, either in a local RoutingTable or doing up to ''maxRequests'' lookup calls
     *
     * @param key         Kademlia key to find node for
     * @param maxRequests Max number of remote requests
     */
    override def findNode(key: Key, maxRequests: Int): F[Option[Node[C]]] =
      routingTable.find(key) match {
        case found @ Some(_) ⇒ (found: Option[Node[C]]).pure[F]

        case None ⇒
          callIterative[String, Unit](
            key,
            n ⇒
              if (n.key === key) EitherT.rightT(())
              else EitherT.leftT("Mismatching node"),
            numToCollect = 1,
            maxNumOfCalls = maxRequests
          ).map(_.headOption.map(_._1))
      }

    /**
     * Perform iterative lookup, see [[RoutingTable.lookupIterative]]
     *
     * @param key Key to lookup
     * @return key's neighborhood
     */
    override def lookupIterative(key: Key, numberOfNodes: Int): F[Seq[Node[C]]] =
      routingTable.lookupIterative(key, numberOfNodes, parallelism, rpc, pingExpiresIn, checkNode)

    /**
     * Performs lookupIterative for a key, and then callIterative for neighborhood.
     * See [[RoutingTable.callIterative]]
     *
     * @param key            Key to call function near
     * @param fn             Function to call
     * @param numToCollect   How many calls are expected to be made
     * @param maxNumOfCalls  Max num of calls before iterations are stopped
     * @param isIdempotentFn If true, there could be more then numToCollect successful calls made
     * @tparam A fn call type
     * @return Sequence of nodes with corresponding successful replies, should be >= numToCollect in case of success
     */
    override def callIterative[E, A](
      key: Key,
      fn: Node[C] ⇒ EitherT[F, E, A],
      numToCollect: Int,
      maxNumOfCalls: Int,
      isIdempotentFn: Boolean = true
    ): F[Seq[(Node[C], A)]] =
      routingTable
        .callIterative(key, fn, numToCollect, parallelism, maxNumOfCalls, isIdempotentFn, rpc, pingExpiresIn, checkNode)
        .map(_.toSeq)

    /**
     * Joins the Kademlia network by a list of known peers. Fails if no join operations performed successfully
     *
     * @param peers Peers contact info
     * @return
     */
    override def join(peers: Seq[C], numberOfNodes: Int): F[Boolean] =
      routingTable.join(peers, rpc, pingExpiresIn, numberOfNodes, checkNode, parallelism).value.map(_.isRight)
  }

}
