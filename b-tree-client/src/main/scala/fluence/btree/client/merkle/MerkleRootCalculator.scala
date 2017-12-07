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
   * calculate merkle tree root. Inserts ''substitutedChecksum'' into element in last position in merkle path.
   * Substitution into the last element occurs at the substitution idx of this element.
   *
   * @param merklePath      Merkle path for getting merkle root
   * @param substitutedChecksum Child's checksum for substitution, it will be inserted to last element into merkle path
   */
  def calcMerkleRoot(merklePath: MerklePath, substitutedChecksum: Array[Byte] = null): Array[Byte] = {
    val optChecksum = Option(substitutedChecksum)
    merklePath.path
      .foldRight(optChecksum) {
        case (nodeProof, prevHash) ⇒
          Some(nodeProof.calcChecksum(cryptoHasher, prevHash))
      }
      .getOrElse(
        optChecksum.map(cryptoHasher.hash)
          .getOrElse(Array.emptyByteArray)
      )
  }

}

object MerkleRootCalculator {
  def apply(cryptoHash: CryptoHasher[Array[Byte], Array[Byte]]): MerkleRootCalculator =
    new MerkleRootCalculator(cryptoHash)
}
