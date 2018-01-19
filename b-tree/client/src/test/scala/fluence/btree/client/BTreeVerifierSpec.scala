package fluence.btree.client

import fluence.btree.common.PutDetails
import fluence.btree.common.merkle.{ GeneralNodeProof, MerklePath }
import fluence.crypto.hash.TestCryptoHasher
import org.scalatest.{ Matchers, WordSpec }

import scala.collection.Searching.{ Found, InsertionPoint }

class BTreeVerifierSpec extends WordSpec with Matchers {

  private val hasher = TestCryptoHasher
  private val verifier = BTreeVerifier(hasher)

  private val key1 = "k1".getBytes
  private val key2 = "k2".getBytes
  private val key3 = "k3".getBytes

  private val val1 = "v1".getBytes
  private val val2 = "v2".getBytes
  private val val3 = "v3".getBytes

  private val hash1 = "h1".getBytes
  private val hash2 = "h2".getBytes
  private val hash3 = "h3".getBytes

  private val keys = Array(key1, key2, key3)
  private val vals = Array(val1, val2, val3)
  private val childsChecksums = Array(hash1, hash2, hash3)

  "checkProof" should {
    "return false" when {
      "if server proof isn't correct (root checking)" in {
        val proof = GeneralNodeProof("H<k1k2_WRONG_KEY>".getBytes, childsChecksums, 1)
        verifier.checkProof(proof, "H<H<k1k2k3>h1h2h3>".getBytes, MerklePath.empty) shouldBe false
      }
      "if server proof isn't correct (second tree lvl checking)" in {
        val proofFromServer = GeneralNodeProof("H<k1k2k3>".getBytes, childsChecksums, 0)
        val clientsProofInMerklePath = GeneralNodeProof("not matter".getBytes, Array(hash1, hash2, hash3), 1)
        verifier.checkProof(proofFromServer, "not matter".getBytes, MerklePath(Seq(clientsProofInMerklePath))) shouldBe false
      }
    }

    "return true" when {
      "if server proof is correct (root checking)" in {
        val proof = GeneralNodeProof("H<k1k2k3>".getBytes, childsChecksums, 1)
        verifier.checkProof(proof, "H<H<k1k2k3>h1h2h3>".getBytes, MerklePath.empty) shouldBe true
      }

      "if server proof is correct for branch (second tree lvl checking)" in {
        val proofFromServer = GeneralNodeProof("H<k1k2k3>".getBytes, childsChecksums, 0)
        val expectedServerProofChecksum = proofFromServer.calcChecksum(hasher, None)
        val expectedServerProofIdx = 1
        val clientsProofInMerklePath =
          GeneralNodeProof("not matter".getBytes, Array(hash1, expectedServerProofChecksum, hash3), expectedServerProofIdx)
        verifier.checkProof(proofFromServer, "not matter".getBytes, MerklePath(Seq(clientsProofInMerklePath))) shouldBe true
      }
      "if server proof is correct for leaf (second tree lvl checking)" in {
        val proofFromServer = GeneralNodeProof(Array.emptyByteArray, childsChecksums, 0)
        val expectedServerProofChecksum = proofFromServer.calcChecksum(hasher, None)
        val expectedServerProofIdx = 1
        val clientsProofInMerklePath =
          GeneralNodeProof("not matter".getBytes, Array(hash1, expectedServerProofChecksum, hash3), expectedServerProofIdx)
        verifier.checkProof(proofFromServer, "not matter".getBytes, MerklePath(Seq(clientsProofInMerklePath))) shouldBe true
      }
    }
  }

  "getBranchProof" should {
    "returns proof for branch details" in {
      val result = verifier.getBranchProof(keys, childsChecksums, 0)
      result.stateChecksum shouldBe "H<k1k2k3>".getBytes
      result.childrenChecksums shouldBe childsChecksums
      result.substitutionIdx shouldBe 0
    }
  }

  "getLeafProof" should {
    "returns proof for branch details" in {
      val result = verifier.getLeafProof(keys, vals)
      result.stateChecksum shouldBe Array.emptyByteArray
      result.childrenChecksums shouldBe Array("H<k1v1>".getBytes, "H<k2v2>".getBytes, "H<k3v3>".getBytes)
      result.substitutionIdx shouldBe -1
    }
  }

  "newMerkleRoot" should {
    "return new merkle root for simple put operation" when {

      "putting into empty tree" in {
        val clientMPath = MerklePath.empty
        val putDetails = PutDetails(key1, val1, InsertionPoint(0))
        val serverMRoot = "H<H<k1v1>>".getBytes

        verifier.newMerkleRoot(clientMPath, putDetails, serverMRoot, wasSplitting = false)
      }
      "insert new value" in {
        val clientMPath = MerklePath(Seq(GeneralNodeProof(Array.emptyByteArray, Array("H<k2v2>".getBytes), 0)))
        val putDetails = PutDetails(key1, val1, InsertionPoint(0))
        val serverMRoot = "H<H<k1v1>H<k2v2>>".getBytes

        val result = verifier.newMerkleRoot(clientMPath, putDetails, serverMRoot, wasSplitting = false)
        result.get shouldBe serverMRoot
      }
      "rewrite old value with new value" in {
        val clientMerklePath = MerklePath(Seq(GeneralNodeProof(Array.emptyByteArray, Array("H<k2v2>".getBytes), 0)))
        val putDetails = PutDetails(key2, val3, Found(0))
        val serverMRoot = "H<H<k2v3>>".getBytes

        val result = verifier.newMerkleRoot(clientMerklePath, putDetails, serverMRoot, wasSplitting = false)
        result.get shouldBe serverMRoot
      }

      "rewrite old value with new value in second tree lvl" in {
        val rootChildsChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k4v4>H<k5v5>>".getBytes)
        val rootProof = verifier.getBranchProof(Array(key2), rootChildsChecksums, 0)
        val leafProof = GeneralNodeProof(Array.emptyByteArray, Array("H<k1v1>".getBytes, "H<k2v2>".getBytes), 1)
        val clientMerklePath = MerklePath(Seq(rootProof, leafProof))
        val putDetails = PutDetails(key2, val3, Found(0))
        val serverMRoot = "H<H<k2>H<H<k1v1>H<k2v3>>H<H<k4v4>H<k5v5>>>".getBytes

        val result = verifier.newMerkleRoot(clientMerklePath, putDetails, serverMRoot, wasSplitting = false)
        result.get shouldBe serverMRoot
      }

      "insert new value in second tree lvl" in {
        val rootChildsChecksums = Array("H<H<k1v1>H<k2v2>>".getBytes, "H<H<k4v4>H<k5v5>>".getBytes)
        val rootProof = verifier.getBranchProof(Array(key2), rootChildsChecksums, 1)
        val leafProof = GeneralNodeProof(Array.emptyByteArray, Array("H<k4v4>".getBytes, "H<k5v5>".getBytes), 0)
        val clientMerklePath = MerklePath(Seq(rootProof, leafProof))
        val putDetails = PutDetails(key3, val3, InsertionPoint(0))
        val serverMRoot = "H<H<k2>H<H<k1v1>H<k2v2>>H<H<k3v3>H<k4v4>H<k5v5>>>".getBytes

        val result = verifier.newMerkleRoot(clientMerklePath, putDetails, serverMRoot, wasSplitting = false)
        result.get shouldBe serverMRoot
      }

    }

    "return None for simple put operation" when {
      "server mRoot != client mRoot" in {
        val clientMerklePath = MerklePath(Seq(GeneralNodeProof(Array.emptyByteArray, Array("H<k2v2>".getBytes), 0)))
        val putDetails = PutDetails(key1, val1, InsertionPoint(0))
        val serverMRoot = "wrong_root".getBytes

        val result = verifier.newMerkleRoot(clientMerklePath, putDetails, serverMRoot, wasSplitting = false)
        result shouldBe None
      }

    }

    "return new merkle root for put operation with tree rebalancing" when {
      // todo implement and write docs
    }

    "return None for put operation with tree rebalancing" when {
      // todo implement and write docs
    }

  }

}
