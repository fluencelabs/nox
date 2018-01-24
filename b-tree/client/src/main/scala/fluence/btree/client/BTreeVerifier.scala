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

package fluence.btree.client

import java.nio.ByteBuffer

import cats.kernel.Eq
import cats.syntax.eq._
import cats.syntax.show._
import fluence.btree.common.BTreeCommonShow._
import fluence.btree.common._
import fluence.btree.common.merkle.{ GeneralNodeProof, MerklePath, MerkleRootCalculator, NodeProof }
import fluence.crypto.hash.CryptoHasher
import org.slf4j.LoggerFactory

import scala.collection.Searching.{ Found, InsertionPoint }

/**
 * Arbiter for checking correctness of Btree server responses.
 * This implementation is thread-safe if corresponded ''cryptoHash'' and ''merkleRootCalculator'' is thread-safe.
 *
 * @param cryptoHasher          Hash provider
 * @param merkleRootCalculator Merkle proof service that allows calculate merkle root from merkle path
 */
class BTreeVerifier(
    cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]],
    merkleRootCalculator: MerkleRootCalculator
) {

  import BTreeVerifier._

  /**
   * Checks 'servers proof' correctness. Calculates proof checksums and compares it with expected checksum.
   *
   * @param serverProof A [[NodeProof]] of branch/leaf for verify from server
   * @param mRoot       The merkle root of server tree
   * @param mPath       The merkle path passed from tree root at this moment
   */
  def checkProof(serverProof: NodeProof, mRoot: Bytes, mPath: MerklePath): Boolean = {

    val calcChecksum = serverProof.calcChecksum(cryptoHasher, None)
    val expectedChecksum = calcExpectedChecksum(mRoot, mPath)

    val verifyingResult = calcChecksum === expectedChecksum
    if (!verifyingResult)
      log.warn(s"Verify branch returns false; expected=${expectedChecksum.show}, calcChecksum=${calcChecksum.show}")
    verifyingResult
  }

  /**
   * Returns [[NodeProof]] for branch details from server.
   *
   * @param keys             Keys of branch for verify
   * @param childsChecksums Childs checksum of branch for verify
   * @param substitutionIdx Next child index.
   */
  def getBranchProof(
    keys: Array[Bytes],
    childsChecksums: Array[Array[Byte]],
    substitutionIdx: Int
  ): GeneralNodeProof = {
    val keysChecksum = cryptoHasher.hash(keys.flatten)
    GeneralNodeProof(keysChecksum, childsChecksums, substitutionIdx)
  }

  /**
   * Returns [[NodeProof]] for branch details from server.
   *
   * @param keys   Keys of leaf for verify
   * @param values Values of leaf for verify
   */
  def getLeafProof(keys: Array[Key], values: Array[Hash]): GeneralNodeProof = {
    val childsChecksums = keys.zip(values).map { case (key, value) ⇒ cryptoHasher.hash(key, value) }
    GeneralNodeProof(Array.emptyByteArray, childsChecksums, -1)
  }

  /**
   * Verifies that server made correct tree modification.
   * Returns Some(newRoot) if server pass verifying, None otherwise.
   * Client can update merkle root if this method returns true.
   *
   * @param clientMPath Clients merkle path
   */
  def newMerkleRoot(
    clientMPath: MerklePath,
    putDetails: PutDetails,
    serverMRoot: Bytes,
    wasSplitting: Boolean
  ): Option[Bytes] = {

    val newMerkleRoot = if (wasSplitting) {
      verifyPutWithRebalancing(clientMPath, putDetails, serverMRoot)
    } else {
      verifySimplePut(clientMPath, putDetails)
    }

    if (newMerkleRoot === serverMRoot) {
      Some(newMerkleRoot)
    } else {
      log.debug(s"New client mRoot=${newMerkleRoot.show} != server mRoot=${serverMRoot.show}")
      None
    }

  }

  /**
   * Verifies that server made correct tree modification without rebalancing.
   * Client can update merkle root if this method returns true.
   * @return Returns Some(newRoot) if server pass verifying, None otherwise.
   */
  private def verifySimplePut(clientMPath: MerklePath, putDetails: PutDetails): Bytes = {

    putDetails match {
      case PutDetails(cipherKey, cipherValue, Found(_)) ⇒
        val keyValChecksum = cryptoHasher.hash(cipherKey, cipherValue)
        merkleRootCalculator.calcMerkleRoot(clientMPath, keyValChecksum)

      case PutDetails(cipherKey, cipherValue, InsertionPoint(_)) ⇒
        val keyValChecksum = cryptoHasher.hash(cipherKey, cipherValue)

        val mPathAfterInserting = clientMPath.path
          .lastOption
          .map {
            case proof @ GeneralNodeProof(_, childrenChecksums, idx) ⇒
              val lastProofAfterInserting =
                proof.copy(childrenChecksums = BytesOps.insertValue(childrenChecksums, keyValChecksum, idx))
              MerklePath(clientMPath.path.init :+ lastProofAfterInserting)
          }
          .getOrElse(MerklePath(Seq(GeneralNodeProof(Array.emptyByteArray, Array(keyValChecksum), 0))))

        merkleRootCalculator.calcMerkleRoot(mPathAfterInserting)
    }
  }

  /**
   * Verifies that server made correct tree modification with tree rebalancing.
   * Client can update merkle root if this method returns true.
   * @return Returns Some(newRoot) if server pass verifying, None otherwise.
   */
  private def verifyPutWithRebalancing(
    clientMPath: MerklePath,
    putDetails: PutDetails,
    serverMRoot: Bytes
  ): Bytes = {

    // todo implement and write tests !!! This methods returns only new merkle root and doesn't verify server response

    serverMRoot
  }

  /**
   * Returns expected checksum of next branch that should be returned from server
   *
   * @param mRoot The merkle root of server tree
   * @param mPath The merkle path already passed from tree root
   */
  private def calcExpectedChecksum(mRoot: Array[Byte], mPath: MerklePath): Bytes = {
    mPath.path.lastOption.map {
      case GeneralNodeProof(_, childrenChecksums, substitutionIdx) ⇒
        childrenChecksums(substitutionIdx)
    }.getOrElse(mRoot)
  }

}

object BTreeVerifier {

  private val log = LoggerFactory.getLogger(getClass)

  def apply(cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]]): BTreeVerifier =
    new BTreeVerifier(cryptoHasher, new MerkleRootCalculator(cryptoHasher))

  // used for comparing two merkle roots
  implicit private val keyEq: Eq[Array[Byte]] = {
    (k1, k2) ⇒ ByteBuffer.wrap(k1).equals(ByteBuffer.wrap(k2))
  }

}
