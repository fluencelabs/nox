package fluence.btree.client.api

/**
 * An interface to calls for a remote MerkleBTree.
 *
 * @tparam F An effect, with MonadError
 * @tparam K The type of key
 * @tparam V The type of value
 */
trait SearchTreeRpc[F[_], K, V] {

  def get(key: K): F[V]

  def put(key: K, value: V): F[V]

  def delete(key: K): F[V]

  // todo discuss traverse, it should be difficult to impl

}
