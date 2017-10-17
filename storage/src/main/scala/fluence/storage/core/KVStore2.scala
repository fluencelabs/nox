package fluence.storage.core

/**
 * Key-value storage api interface.
 *
 * @tparam K - type of key
 * @tparam V - type of stored value
 * @tparam D - type of dataSet, dataSet is a group for keys in other words keyspace
 * @tparam F - effect, container for returning value
 * @tparam FS - effect, container for returning stream of value, aka cursor
 */
trait KVStore2[K, V, D, F[_], FS[_]] {

  /**
   * Get stored value for specified dataset and key.
   * @param ds - dataSet is a group for keys in other words keyspace
   * @param key - the key retrieve the value.
   */
  def get(ds: D, key: K): F[V]

  /**
   * Put the database entry for "key" to "value".
   * @param ds - dataSet is a group for keys in other words keyspace
   * @param key - the specified key to be inserted
   * @param value - the value associated with the specified key
   */
  def put(ds: D, key: K, value: V): F[Boolean]

  /**
   * Remove pair (K, V) for specified dataSet and key.
   * @param ds - dataSet is a group for keys in other words keyspace
   * @param key - key to delete within database
   */
  def remove(ds: D, key: K): F[Boolean]

  /**
   * Return all pairs (K, V) for specified dataSet.
   * @param ds - dataSet
   */
  def traverse(ds: D): FS[(K, V)]

}
