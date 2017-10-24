package fluence.node.storage

/**
 * Key-value storage api interface.
 *
 * @tparam K - type of keys
 * @tparam V - type of stored values
 * @tparam F - box for returning value
 * @tparam FS - box for returning stream(cursor) of value
 */
trait KVStore[K, V, F[_], FS[_]] {

  /**
   * Gets stored value for specified key.
   * @param key - the key retrieve the value.
   */
  def get(key: K): F[V]

  /**
   * Puts key value pair (K, V).
   * Update existing value if it's present.
   * @param key - the specified key to be inserted
   * @param value - the value associated with the specified key
   */
  def put(key: K, value: V): F[Unit]

  /**
   * Removes pair (K, V) for specified key.
   * @param key - key to delete within database
   */
  def remove(key: K): F[Unit]

  /**
   * Return all pairs (K, V) for specified dataSet.
   * @return cursor to founded pairs (K,V)
   */
  def traverse(): FS[(K, V)]

}
