package fluence.kad

import scala.language.higherKinds

/**
 * An interface to Kademlia-related calls for a remote node
 * @tparam F An effect, with MonadError
 * @tparam C Type for contact data
 */
trait KademliaRPC[F[_], C] {
  /**
   * Ping the contact, get its actual Node status, or fail
   * @return
   */
  def ping(): F[Node[C]]

  /**
   * Perform a local lookup for a key, return K closest known nodes
   * @param key Key to lookup
   * @return
   */
  def lookup(key: Key, numberOfNodes: Int): F[Seq[Node[C]]]

  /**
   * Perform an iterative lookup for a key, return K closest known nodes
   * @param key Key to lookup
   * @return
   */
  def lookupIterative(key: Key, numberOfNodes: Int): F[Seq[Node[C]]]

}
