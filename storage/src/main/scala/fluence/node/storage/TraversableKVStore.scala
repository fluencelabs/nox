package fluence.node.storage

import scala.language.higherKinds

/**
 * Traversable Key-value storage api interface.
 *
 * @tparam FS A box for returning stream(cursor) of value
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
trait TraversableKVStore[FS[_], K, V] {

  /**
   * Return all pairs (K, V) for specified dataSet.
   * @return cursor to founded pairs (K,V)
   */
  def traverse(): FS[(K, V)]
}
