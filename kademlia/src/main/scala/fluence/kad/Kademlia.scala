package fluence.kad

import cats.{ MonadError, Parallel }
import cats.syntax.applicative._
import cats.syntax.functor._
import RoutingTable._

import scala.concurrent.duration.Duration
import scala.language.higherKinds

/**
 * Kademlia interface for current node and all Kademlia-related RPC calls, both incoming and outgoing
 * @param parallelism Parallelism factor (named Alpha in paper)
 * @param pingTimeout Duration to avoid too frequent ping requests, used in [[Bucket.update()]]
 * @param ME Monad error
 * @tparam F Effect
 * @tparam C Contact info
 */
abstract class Kademlia[F[_], C](
    val nodeId: Key,
    parallelism: Int,
    val pingTimeout: Duration
)(implicit ME: MonadError[F, Throwable], P: Parallel[F, F], BW: Bucket.WriteOps[F, C], SW: Siblings.WriteOps[F, C]) {
  self ⇒

  /**
   * Returns a network wrapper around a contact C, allowing querying it with Kademlia protocol
   * @param contact Description on how to connect to remote node
   * @return
   */
  def rpc(contact: C): KademliaRPC[F, C]

  /**
   * How to promote this node to others
   * @return
   */
  def ownContact: F[Node[C]]

  /**
   * Update RoutingTable with a freshly seen node
   * @param node Discovered node, known to be alive and reachable
   * @return
   */
  def update(node: Node[C]): F[Boolean] =
    nodeId.update(node, rpc, pingTimeout)

  /**
   * Returns KademliaRPC instance to handle incoming RPC requests
   * @return
   */
  val handleRPC: KademliaRPC[F, C] = new KademliaRPC[F, C] {

    /**
     * Respond for a ping with node's own contact data
     * @return
     */
    override def ping(): F[Node[C]] =
      ownContact

    /**
     * Perform local lookup
     * @param key Key to lookup
     * @return
     */
    override def lookup(key: Key, numberOfNodes: Int): F[Seq[Node[C]]] =
      ().pure[F].map(_ ⇒ nodeId.lookup(key).take(numberOfNodes))

    /**
     * Perform iterative lookup
     * @param key Key to lookup
     * @return
     */
    override def lookupIterative(key: Key, numberOfNodes: Int): F[Seq[Node[C]]] =
      nodeId.lookupIterative(key, numberOfNodes, parallelism, rpc, pingTimeout)
  }

  /**
   * Joins the Kademlia network by a list of known peers. Fails if no join operations performed successfully
   * @param peers Peers contact info
   * @return
   */
  def join(peers: Seq[C], numberOfNodes: Int): F[Unit] =
    nodeId.join(peers, rpc, pingTimeout, numberOfNodes)
}
