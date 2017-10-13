package fluence.kad

import cats.MonadError
import cats.data.StateT
import cats.syntax.applicative._
import cats.syntax.functor._
import cats.syntax.flatMap._

import scala.concurrent.duration.Duration
import scala.language.higherKinds

/**
 * Kademlia interface for current node and all Kademlia-related RPC calls, both incoming and outgoing
 * @param Alpha Parallelism factor
 * @param K Max size of bucket, max size of siblings, number of lookup results returned
 *          @param pingTimeout Duration to avoid too frequent ping requests, used in [[Bucket.update()]]
 * @param ME Monad error
 * @tparam F Effect
 * @tparam C Contact info
 */
abstract class Kademlia[F[_], C](
    val Alpha:       Int,
    val K:           Int,
    val pingTimeout: Duration

)(implicit ME: MonadError[F, Throwable]) {
  self ⇒

  /**
   * Run some stateful operation, possibly mutating it
   * @param mod Operation
   * @tparam T Return type
   * @return
   */
  protected def run[T](mod: StateT[F, RoutingTable[C], T]): F[T]

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
  def ownContact: Node[C]

  /**
   * Update RoutingTable with a freshly seen node
   * @param node Discovered node, known to be alive and reachable
   * @return
   */
  def update(node: Node[C]): F[Unit] =
    run(RoutingTable.update(node, rpc, pingTimeout))

  /**
   * Returns KademliaRPC instance to handle incoming RPC requests
   * @param from Node that have made a request, if it's verified, or null
   * @return
   */
  def handleRPC(from: Node[C] = null): KademliaRPC[F, C] = new KademliaRPC[F, C] {

    // Update RoutingTable with a sender node
    private lazy val updateFrom: F[Unit] =
      Option(from).fold(().pure[F])(fromNode ⇒ update(fromNode))

    /**
     * Respond for a ping with node's own contact data
     * @return
     */
    override def ping(): F[Node[C]] =
      updateFrom.map(_ ⇒ ownContact)

    /**
     * Perform local lookup
     * @param key Key to lookup
     * @return
     */
    override def lookup(key: Key): F[Seq[Node[C]]] =
      for {
        _ ← updateFrom
        nodes ← run(RoutingTable.lookup[F, C](key))
      } yield nodes.take(K)

    /**
     * Perform iterative lookup
     * @param key Key to lookup
     * @return
     */
    override def lookupIterative(key: Key): F[Seq[Node[C]]] =
      for {
        _ ← updateFrom
        nodes ← run(RoutingTable.lookupIterative[F, C](key, K, Alpha, rpc, pingTimeout))
      } yield nodes
  }

  /**
   * Joins the Kademlia network by a list of known peers. Fails if no join operations performed successfully
   * @param peers Peers contact info
   * @return
   */
  def join(peers: Seq[C]): F[Unit] =
    run(RoutingTable.join(peers, rpc, pingTimeout))
}