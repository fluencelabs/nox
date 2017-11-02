package fluence.btree.core

/**
 * Simple Search Tree interface.
 *
 * @tparam K key type
 * @tparam V value type
 * @tparam R result type
 * @tparam F result box (effect)
 */
trait SearchTree[K, V, R, F[_]] {

  def get(key: K): F[R]

  def put(key: K, value: V): F[R]

  def delete(key: K): F[R]

}
