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
 * Contains all information needed for recalculating checksum of node with substituting child's checksum, for verifying
 * child's checksum or recalculating checksum of the node.
 */
sealed trait NodeProof {

  /**
   * Calculates checksum of current ''NodeProof''.
   *
   * @param cryptoHasher Hash service uses for calculating nodes checksums.
   * @param checksumForSubstitution Child's checksum for substitution to ''childrenChecksums''
   * @return Checksum of current ''NodeProof''
   */
  def calcChecksum(
    cryptoHasher: CryptoHasher[Array[Byte], Hash],
    checksumForSubstitution: Option[Hash]
  ): Hash

}

/**
 * ''GeneralNodeProof'' contains 2 part:
 *    ''stateChecksum'' - a checksum of node state, which is not related to the checksums of node children
 *    ''childrenChecksums'' - an array of childs checksums, which participates in the substitution
 * GenerateNodeProof's checksum is a hash of ''stateChecksum'' and ''childrenChecksums''
 *
 * For leaf ''childrenChecksums'' is sequence with checksums of each 'key-value' pair of this leaf and ''substitutionIdx''
 * is index for inserting checksum of 'key-value' pair.
 *
 * For branch ''childrenChecksums'' is sequence with checksums for each children of this branch and ''substitutionIdx''
 * is index for inserting checksum of child.
 *
 * @param stateChecksum     A checksum of node state, which is not related to the checksums of children
 * @param childrenChecksums Checksums of all node children
 * @param substitutionIdx   Index for substitution some child checksum to ''childrenChecksums''
 */
case class GeneralNodeProof(
    stateChecksum: Hash,
    childrenChecksums: Array[Hash],
    substitutionIdx: Int
) extends NodeProof {

  override def calcChecksum(
    cryptoHasher: CryptoHasher[Array[Byte], Hash],
    checksumForSubstitution: Option[Hash]
  ): Hash = {

    checksumForSubstitution match {
      case None ⇒
        // if checksum for substitution isn't defined just calculate node checksum
        val checksum = stateChecksum.concat(childrenChecksums)
        if (checksum.isEmpty) checksum else cryptoHasher.hash(checksum.bytes)
      case Some(checksum) ⇒
        // if checksum is defined substitute it to childsChecksum and calculate node checksum
        val updatedArray = childrenChecksums.rewriteValue(checksum, substitutionIdx)
        cryptoHasher.hash(stateChecksum.concat(updatedArray).bytes)
    }

  }

  override def toString: String = s"NodeProof(stateChecksum=$stateChecksum, " +
    s"childrenChecksums=${childrenChecksums.mkString(",")}, substitutionIdx=$substitutionIdx)"
}
