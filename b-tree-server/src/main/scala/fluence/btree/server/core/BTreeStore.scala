package fluence.btree.server.core

/**
 * BTree persistence store.
 *
 * @tparam Id The type of node id
 * @tparam Node The type of node
 * @tparam F - Some box for returning value
 */
trait BTreeStore[Id, Node, F[_]] {

  /**
   * Gets stored node for specified id.
   * @param nodeId - id of stored the node.
   */
  def get(nodeId: Id): F[Node]

  /**
   * Store specified node with specified id.
   * Rewrite existing value if it's present.
   * @param nodeId - the specified node id to be inserted
   * @param node - the node associated with the specified node id
   */
  def put(nodeId: Id, node: Node): F[Unit]

  // todo: additional methods like 'remove' will be created on demand

}
