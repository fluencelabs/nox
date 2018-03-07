/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.btree.common.merkle

import fluence.btree.common.Hash
import fluence.crypto.hash.CryptoHasher

/**
 * Merkle proof service that allows calculate merkle root from merkle path.
 * This implementation is thread-safe if corresponded cryptoHash is thread-safe.
 *
 * @param cryptoHasher Hash provider
 */
class MerkleRootCalculator(cryptoHasher: CryptoHasher[Array[Byte], Hash]) {

  /**
   * Calculates new merkle root from merkle path. Folds merkle path from the right to the left and
   * calculate merkle tree root. Inserts ''substitutedChecksum'' into element in last position in merkle path.
   * Substitution into the last element occurs at the substitution idx of this element.
   *
   * @param merklePath      Merkle path for getting merkle root
   * @param substitutedChecksum Child's checksum for substitution, it will be inserted to last element into merkle path
   */
  def calcMerkleRoot(merklePath: MerklePath, substitutedChecksum: Option[Hash] = None): Hash = {
    merklePath.path
      .foldRight(substitutedChecksum) {
        case (nodeProof, prevHash) ⇒
          Some(nodeProof.calcChecksum(cryptoHasher, prevHash))
      }
      .getOrElse(
        substitutedChecksum.map(cs ⇒ cryptoHasher.hash(cs.bytes))
          .getOrElse(Hash.empty)
      )
  }

}

object MerkleRootCalculator {
  def apply(cryptoHash: CryptoHasher[Array[Byte], Hash]): MerkleRootCalculator =
    new MerkleRootCalculator(cryptoHash)
}
