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
   */
  def ping(): F[Node[C]]

  /**
   * Perform a local lookup for a key, return K closest known nodes
   * @param key Key to lookup
   */
  def lookup(key: Key, numberOfNodes: Int): F[Seq[Node[C]]]

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key
   * @param key Key to lookup
   */
  def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): F[Seq[Node[C]]]

  /**
   * Perform an iterative lookup for a key, return K closest known nodes
   * @param key Key to lookup
   */
  // TODO: it's used only in [[RoutingTable.WriteOps.join()]]; gives the way to perform a big number of requests
  // on distant node with RPC call; could be unsafe to expose with RPC
  def lookupIterative(key: Key, numberOfNodes: Int): F[Seq[Node[C]]]

}
