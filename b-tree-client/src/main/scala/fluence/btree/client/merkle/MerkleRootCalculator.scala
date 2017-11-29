package fluence.btree.client.merkle

import fluence.hash.CryptoHasher

/**
 * Merkle proof service that allows calculate merkle root from merkle path.
 * This implementation is thread-safe if corresponded cryptoHash is thread-safe.
 *
 * @param cryptoHasher Hash provider
 */
class MerkleRootCalculator(cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]]) {

  /**
   * Calculates new merkle root from merkle path. Folds merkle path from the right to the left and
   * calculate merkle tree root. Inserts ''hashForChecking'' into element in last position in merkle path.
   * Substitution into the last element occurs at the substitution idx of this element.
   *
   * @param merklePath      Merkle path for getting merkle root
   * @param substitutedChecksum Child's checksum for substitution, it will be inserted to last element into merkle path
   */
  def calcMerkleRoot(merklePath: MerklePath, substitutedChecksum: Array[Byte]): Array[Byte] = {
    if (merklePath.path.isEmpty) {
      cryptoHasher.hash(substitutedChecksum)
    } else {
      merklePath.path
        .foldRight(substitutedChecksum) {
          case (NodeProof(childrenHashes, substitutionIdx), prevHash) ⇒
            val updatedArray = rewriteValue(childrenHashes, substitutionIdx, prevHash)
            cryptoHasher.hash(updatedArray.flatten)
        }
    }
  }

  /**
   * Calculates merkle root from merkle path. Folds merkle path from the right to the left and calculate merkle tree root.
   *
   * @param merklePath Merkle path for getting merkle root
   */
  def calcMerkleRoot(merklePath: MerklePath): Array[Byte] = {
    merklePath.path
      .foldRight(Option.empty[Array[Byte]]) {
        case (NodeProof(childrenHashes, substitutionIdx), prevHash) ⇒
          val newHash = prevHash match {
            case None ⇒
              cryptoHasher.hash(childrenHashes.flatten)
            case Some(hash) ⇒
              val updatedArray = rewriteValue(childrenHashes, substitutionIdx, hash)
              cryptoHasher.hash(updatedArray.flatten)
          }
          Some(newHash)
      }.getOrElse(Array.emptyByteArray)
  }

  /**
   * Returns updated copy of array with the updated element for ''insIdx'' index.
   * We choose variant with array copying for prevent changing input parameters.
   * Work with mutable structures is more error-prone. It may be changed in the future by performance reason.
   */
  private def rewriteValue(array: Array[Array[Byte]], insIdx: Int, insElem: Array[Byte]) = {
    // todo perhaps, more optimal implementation might be needed with array mutation in the future
    val newArray = Array.ofDim[Array[Byte]](array.length)
    Array.copy(array, 0, newArray, 0, array.length)
    newArray(insIdx) = insElem
    newArray
  }

}

object MerkleRootCalculator {
  def apply(cryptoHash: CryptoHasher[Array[Byte], Array[Byte]]): MerkleRootCalculator =
    new MerkleRootCalculator(cryptoHash)
}
