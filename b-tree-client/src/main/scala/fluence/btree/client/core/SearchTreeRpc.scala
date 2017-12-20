package fluence.btree.client.core

/**
 * An interface to calls for a remote MerkleBTree.
 *
 * @tparam F An effect, with MonadError
 * @tparam K The type of plain text key
 * @tparam V The type of plain text value
 */
trait SearchTreeRpc[F[_], K, V] {

  def get(key: K): F[Option[V]]

  def put(key: K, value: V): F[Option[V]]

  def delete(key: K): F[Option[V]]

  // todo discuss traverse, it should be difficult to impl

}
