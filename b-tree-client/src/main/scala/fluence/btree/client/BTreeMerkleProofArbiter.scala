package fluence.btree.client

import java.nio.ByteBuffer

import cats.kernel.Eq
import cats.syntax.eq._
import cats.syntax.show._
import fluence.btree.client.merkle.MerkleRootCalculator
import fluence.hash.CryptoHasher
import org.slf4j.LoggerFactory

/**
 * Arbiter for checking correctness of Btree responses.
 * This implementation is thread-safe if corresponded ''merkleProof'' and ''cryptoHash'' is thread-safe.
 *
 * @param merkleProof Manager poof service for calculating merkle root
 * @param cryptoHash  Hash provider
 */
class BTreeMerkleProofArbiter(merkleProof: MerkleRootCalculator, cryptoHash: CryptoHasher[Array[Byte], Array[Byte]]) {

  import BTreeMerkleProofArbiter._

  /**
   * Verifies state of Btree and returned values.
   * Checks that remote btree state is corresponds client state and returned result is correct.
   *
   * @param readResult Response from MerkleBTree
   * @param merkleRoot  Client merkle root
   */
  def verifyGet(readResult: ReadResults, merkleRoot: Array[Byte]): Boolean = {
    val calculatedRoot = readResult.value match {
      case Some(value) ⇒ // case when key was found
        merkleProof.calcMerkleRoot(readResult.proof, cryptoHash.hash(readResult.key, value))
      case None ⇒ // case when key wasn't found
        merkleProof.calcMerkleRoot(readResult.proof)
    }
    val verifyingResult = calculatedRoot === merkleRoot
    if (!verifyingResult)
      log.warn(s"VerifyPut returns false for oldRoot=${merkleRoot.show}, calcRoot=${calculatedRoot.show}, result=$readResult")

    verifyingResult
  }

  /**
   * Verifies state of Btree before doing any write operation.
   * Checks that remote btree state is corresponds client state before doing mutation.
   *
   * @param writeResult Response from MerkleBTree
   * @param merkleRoot  Client merkle root
   */
  def verifyPut(writeResult: WriteResults, merkleRoot: Array[Byte]): Boolean = {
    val calculatedRoot = merkleProof.calcMerkleRoot(writeResult.oldStateProof)
    val verifyingResult = merkleRoot.isEmpty || calculatedRoot === merkleRoot
    if (!verifyingResult)
      log.warn(s"VerifyPut returns false for oldRoot=${merkleRoot.show}, calcRoot=${calculatedRoot.show}, result=$writeResult")
    verifyingResult
  }

  /**
   * Calculates new merkle root. Substitutes hash of returned 'key-value' to ''newStateProof''.
   * Note that before recalculating merkle root client must be satisfied that returned key and value the same as requested.
   */
  def calcNewMerkleRoot(result: WriteResults): Array[Byte] =
    merkleProof.calcMerkleRoot(result.newStateProof, cryptoHash.hash(result.key, result.value))

}

object BTreeMerkleProofArbiter {

  private val log = LoggerFactory.getLogger(getClass)

  def apply(cryptoHash: CryptoHasher[Array[Byte], Array[Byte]]): BTreeMerkleProofArbiter =
    new BTreeMerkleProofArbiter(new MerkleRootCalculator(cryptoHash), cryptoHash)

  // used for comparing two merkle roots
  implicit private val keyEq: Eq[Array[Byte]] = {
    (k1, k2) ⇒ ByteBuffer.wrap(k1).equals(ByteBuffer.wrap(k2))
  }

}
