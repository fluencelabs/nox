package fluence.merkle.ops

trait MerkleOperations[T] {

  /**
   * Default value for leafs in a tree.
   *
   * @param chunkSize leafs depends on chunk's size
   */
  def defaultLeaf(chunkSize: Int): T

  /**
   * Calculates hash.
   *
   * @param t value to hash
   * @return hash
   */
  def hash(t: T): T

  /**
   * Concatenates two values.
   *
   * @return result of concatenation
   */
  def concatenate(l: T, r: T): T
}
