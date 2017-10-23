package fluence.kad

import cats.{ Id, MonadError }
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
 * @param pingTimeout Duration to avoid too frequent ping requests, used in [[Bucket.update()]]
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
  protected def run[T](mod: StateT[F, RoutingTable[C], T], l: String): F[T]

  /**
   * Non-blocking read request
   * @param getter Getter for routing table
   * @tparam T Return type
   * @return
   */
  protected def read[T](getter: RoutingTable[C] ⇒ T): F[T]

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
  def update(node: Node[C]): F[Unit] =
    read(_.initialized).flatMap {
      case true ⇒
        run(RoutingTable.update(node, rpc, pingTimeout), "update")
      case false ⇒
        ().pure[F]
    }

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
      for {
        nodes ← read(rt ⇒ RoutingTable.lookup[Id, C](key).run(rt)._2)
      } yield nodes.take(numberOfNodes)

    /**
     * Perform iterative lookup
     * @param key Key to lookup
     * @return
     */
    override def lookupIterative(key: Key, numberOfNodes: Int): F[Seq[Node[C]]] =
      read(_.initialized).flatMap {
        case true ⇒
          run(RoutingTable.lookupIterative[F, C](key, numberOfNodes, Alpha, rpc, pingTimeout), "lookup iterative")
        case false ⇒
          Seq.empty[Node[C]].pure[F]
      }
  }

  /**
   * Joins the Kademlia network by a list of known peers. Fails if no join operations performed successfully
   * @param peers Peers contact info
   * @return
   */
  def join(peers: Seq[C]): F[Unit] =
    run(RoutingTable.join(peers, rpc, pingTimeout, K), "join")
}