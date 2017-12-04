package fluence.kad

import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.monoid._
import cats.syntax.order._
import cats.{ MonadError, Parallel }
import fluence.kad.RoutingTable._

import scala.concurrent.duration.Duration
import scala.language.higherKinds

/**
 * Kademlia interface for current node and all Kademlia-related RPC calls, both incoming and outgoing
 *
 * @param parallelism   Parallelism factor (named Alpha in paper)
 * @param pingExpiresIn Duration to avoid too frequent ping requests, used in [[Bucket.update()]]
 * @param ME            Monad error
 * @tparam F Effect
 * @tparam C Contact info
 */
abstract class Kademlia[F[_], C](
    val nodeId: Key,
    parallelism: Int,
    val pingExpiresIn: Duration
)(implicit ME: MonadError[F, Throwable], P: Parallel[F, F], BW: Bucket.WriteOps[F, C], SW: Siblings.WriteOps[F, C]) {
  self ⇒

  /**
   * Returns a network wrapper around a contact C, allowing querying it with Kademlia protocol
   *
   * @param contact Description on how to connect to remote node
   * @return
   */
  def rpc(contact: C): KademliaRPC[F, C]

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
    nodeId.update(node, rpc, pingExpiresIn)

  /**
   * @return KademliaRPC instance to handle incoming RPC requests
   */
  val handleRPC: KademliaRPC[F, C] = new KademliaRPC[F, C] {

    /**
     * Respond for a ping with node's own contact data
     *
     * @return
     */
    override def ping(): F[Node[C]] =
      ownContact

    /**
     * Perform a lookup in local RoutingTable
     *
     * @param key Key to lookup
     * @return locally known neighborhood
     */
    override def lookup(key: Key, numberOfNodes: Int): F[Seq[Node[C]]] =
      ().pure[F].map(_ ⇒ nodeId.lookup(key).take(numberOfNodes))

    /**
     * Perform a lookup in local RoutingTable for a key,
     * return `numberOfNodes` closest known nodes, going away from the second key
     *
     * @param key Key to lookup
     */
    override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): F[Seq[Node[C]]] =
      ().pure[F].map(_ ⇒
        nodeId.lookupAway(key, moveAwayFrom)
          .take(numberOfNodes)
      )

    /**
     * Perform iterative lookup, see [[RoutingTable.WriteOps.lookupIterative]]
     *
     * @param key Key to lookup
     * @return key's neighborhood
     */
    override def lookupIterative(key: Key, numberOfNodes: Int): F[Seq[Node[C]]] =
      nodeId.lookupIterative(key, numberOfNodes, parallelism, rpc, pingExpiresIn)
  }

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
  def callIterative[A](key: Key, fn: Node[C] ⇒ F[A], numToCollect: Int, maxNumOfCalls: Int, isIdempotentFn: Boolean = true): F[Seq[(Node[C], A)]] =
    nodeId.callIterative(key, fn, numToCollect, parallelism, maxNumOfCalls, isIdempotentFn, rpc, pingExpiresIn)

  /**
   * Joins the Kademlia network by a list of known peers. Fails if no join operations performed successfully
   *
   * @param peers Peers contact info
   * @return
   */
  def join(peers: Seq[C], numberOfNodes: Int): F[Unit] =
    nodeId.join(peers, rpc, pingExpiresIn, numberOfNodes)
}
