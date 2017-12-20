package fluence.btree.client

import java.nio.ByteBuffer

import cats.kernel.Eq
import cats.syntax.eq._
import cats.syntax.show._
import fluence.btree.client.common.BytesOps
import fluence.btree.client.merkle.{ GeneralNodeProof, MerklePath, MerkleRootCalculator, NodeProof }
import fluence.btree.client.network._
import fluence.hash.CryptoHasher
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
  def getLeafProof(keys: Array[Key], values: Array[Value]): GeneralNodeProof = {
    val childsChecksums = keys.zip(values).map { case (key, value) ⇒ cryptoHasher.hash(key, value) }
    GeneralNodeProof(Array.emptyByteArray, childsChecksums, -1)
  }

  /**
   * Verifies that server made correct tree modification.
   * Returns Some(newRoot) if server pass verifying, None otherwise.
   * Client can update merkle root if this method returns true.
   *
   * @param merklePath  Clients merkle path
   * @param putRequest  Clients put request submitted to server
   * @param putResponse A response from server, for verifying put operation
   */
  def newMerkleRoot(merklePath: MerklePath, putRequest: PutRequest, putResponse: VerifyChanges): Option[Bytes] = {
    putResponse match {
      case putResponse: VerifySimplePutResponse ⇒
        verifyPut(merklePath, putRequest, putResponse)
      case putResponse: VerifyPutWithRebalancingResponse ⇒
        verifyPut(merklePath, putRequest, putResponse)
    }
  }

  /**
   * Verifies that server made correct tree modification without rebalancing.
   * Client can update merkle root if this method returns true.
   * @return Returns Some(newRoot) if server pass verifying, None otherwise.
   */
  private def verifyPut(mPath: MerklePath, putReq: PutRequest, putRes: VerifySimplePutResponse): Option[Bytes] = {

    val newMerkleRoot =
      putReq match {
        case PutRequest(cipherKey, cipherValue, Found(_)) ⇒
          val keyValChecksum = cryptoHasher.hash(cipherKey, cipherValue)
          merkleRootCalculator.calcMerkleRoot(mPath, keyValChecksum)

        case PutRequest(cipherKey, cipherValue, InsertionPoint(_)) ⇒
          val keyValChecksum = cryptoHasher.hash(cipherKey, cipherValue)

          val mPathAfterInserting = mPath.path
            .lastOption
            .map {
              case proof @ GeneralNodeProof(_, childrenChecksums, idx) ⇒
                val lastProofAfterInserting =
                  proof.copy(childrenChecksums = BytesOps.insertValue(childrenChecksums, keyValChecksum, idx))
                MerklePath(mPath.path.init :+ lastProofAfterInserting)
            }
            .getOrElse(MerklePath(Seq(GeneralNodeProof(Array.emptyByteArray, Array(keyValChecksum), 0))))

          merkleRootCalculator.calcMerkleRoot(mPathAfterInserting)
      }

    if (newMerkleRoot === putRes.merkleRoot) {
      Some(newMerkleRoot)
    } else {
      log.debug(s"New client mRoot=${newMerkleRoot.show} != server mRoot=${putRes.merkleRoot.show}")
      None
    }

  }

  /**
   * Verifies that server made correct tree modification with tree rebalancing.
   * Client can update merkle root if this method returns true.
   * @return Returns Some(newRoot) if server pass verifying, None otherwise.
   */
  private def verifyPut(mPath: MerklePath, putReq: PutRequest, putRes: VerifyPutWithRebalancingResponse): Option[Bytes] = {

    // todo implement and write tests !!! This methods returns only new merkle root and doesn't verify server response

    Some(merkleRootCalculator.calcMerkleRoot(putRes.merklePath))
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
