package fluence.btree.client

import fluence.btree.client.merkle.{ GeneralNodeProof, MerklePath, NodeProof }
import fluence.hash.TestCryptoHasher
import org.scalatest.{ Matchers, WordSpec }

class BTreeMerkleProofInspectorSpec extends WordSpec with Matchers {

  private val hasher = TestCryptoHasher
  private val inspector = BTreeMerkleProofInspector(hasher)

  private val key1 = "k1".getBytes
  private val val1 = "v1".getBytes
  private val key2 = "k2".getBytes
  private val val2 = "v2".getBytes
  private val key3 = "k3".getBytes
  private val val3 = "v3".getBytes

  private val key4 = "k4".getBytes
  private val key6 = "k6".getBytes

  /* Generate leaf prof */
  private def createLeafProof(substitutionIdx: Int, keySuffix: Int*): GeneralNodeProof = {
    val kvHashes = keySuffix.map(idx ⇒ hasher.hash(s"k${idx}v${idx}".getBytes)).toArray
    GeneralNodeProof(Array.emptyByteArray, kvHashes, substitutionIdx)
  }

  /* Generate branch prof */
  private def createBranchProof(stateChecksum: Array[Byte], substitutionIdx: Int, childrenHashes: String*): GeneralNodeProof = {
    GeneralNodeProof(stateChecksum, childrenHashes.map(_.getBytes).toArray, substitutionIdx)
  }

  private def merklePath(nodeProofs: NodeProof*) =
    MerklePath(nodeProofs)

  "verifyGet" should {
    "return true" when {
      "merkle path hash one element, key found " in {
        val result = ReadResults(key1, Some(val1), merklePath(createLeafProof(substitutionIdx = 0, 1, 2)))
        inspector.verifyGet(result, "H<H<k1v1>H<k2v2>>".getBytes) shouldBe true
      }
      "merkle path hash one element, key not found " in {
        val result = ReadResults(key3, None, merklePath(createLeafProof(substitutionIdx = -1, 1, 2, 3)))
        inspector.verifyGet(result, "H<H<k1v1>H<k2v2>H<k3v3>>".getBytes) shouldBe true
      }
      "merkle path hash two elements, key found " in {
        val leafProof = createLeafProof(substitutionIdx = 2, 1, 2, 3)
        val leftLeafHash = "H<H<k1v1>H<k2v2>H<k3v3>>"
        val rightLeafHash = "H<H<k5v5>H<k6v6>>"
        val parentBranch = createBranchProof(key3, substitutionIdx = 0, leftLeafHash, rightLeafHash)
        val result = ReadResults(key3, Some(val3), merklePath(parentBranch, leafProof))
        inspector.verifyGet(result, hasher.hash(key3, parentBranch.childrenChecksums.flatten)) shouldBe true
      }
      "merkle path hash two elements, key not found " in {
        val leafProof = createLeafProof(substitutionIdx = -1, 1, 2)
        val leftLeafHash = "H<H<k1v1>H<k2v2>>"
        val rightLeafHash = "H<H<k5v5>H<k6v6>>"
        val parentTree = createBranchProof(key3, substitutionIdx = 0, leftLeafHash, rightLeafHash)
        val result = ReadResults(key3, None, merklePath(parentTree, leafProof))
        inspector.verifyGet(result, hasher.hash(key3, parentTree.childrenChecksums.flatten)) shouldBe true
      }
      "merkle path hash many elements, key found" in {
        val leafProof = createLeafProof(substitutionIdx = 0, 3, 4)
        val leftLeafHash = "H<H<k1v1>H<k2v2>>"
        val midLeafHash = "H<H<k3v3>H<k4v4>>"
        val rightLeafHash = "H<H<k5v5>H<k6v6>>"
        val parent1Tree = createBranchProof(hasher.hash(key2, key4), substitutionIdx = 1, leftLeafHash, midLeafHash, rightLeafHash)
        val parent2Tree = createBranchProof(hasher.hash(key6), substitutionIdx = 0, s"H<H<k2k4>$leftLeafHash$midLeafHash$rightLeafHash>", "h<...>")
        val result = ReadResults(key3, Some(val3), merklePath(parent2Tree, parent1Tree, leafProof))
        inspector.verifyGet(result, parent2Tree.calcChecksum(hasher, None)) shouldBe true
      }
      "merkle path hash many elements, key not found" in {
        val leafProof = createLeafProof(substitutionIdx = 0, 4)
        val leftLeafHash = "H<H<k1v1>H<k2v2>>"
        val midLeafHash = "H<H<k4v4>>"
        val rightLeafHash = "H<H<k5v5>H<k6v6>>"
        val parent1Tree = createBranchProof(hasher.hash(key2, key4), substitutionIdx = 1, leftLeafHash, midLeafHash, rightLeafHash)
        val parent2Tree = createBranchProof(hasher.hash(key6), substitutionIdx = 0, s"H<H<k2k4>$leftLeafHash$midLeafHash$rightLeafHash>", "h<...>")
        val result = ReadResults(key3, None, merklePath(parent2Tree, parent1Tree, leafProof))
        inspector.verifyGet(result, parent2Tree.calcChecksum(hasher, None)) shouldBe true
      }
    }

    "return false always" when {
      "merkle root didn't match" in {
        val result = ReadResults(key1, Some(val1), merklePath(createLeafProof(0, 1, 2)))
        inspector.verifyGet(result, "H<wrong root>".getBytes) shouldBe false
      }
    }
  }

  "verifyPut" should {
    "return true" when {
      "merkle path hash one element, element was inserted" in {
        val result = WriteResults(key1, val1, merklePath(createLeafProof(substitutionIdx = 1, 2, 3)), null)
        inspector.verifyPut(result, "H<H<k2v2>H<k3v3>>".getBytes) shouldBe true
      }
      "merkle path hash one element, element was updated" in {
        val result = WriteResults(key1, val1, merklePath(createLeafProof(substitutionIdx = 0, 1, 2)), null)
        inspector.verifyPut(result, "H<H<k1v1>H<k2v2>>".getBytes) shouldBe true
      }

      "merkle path hash many elements, element was inserted" in {

        val leafProof = createLeafProof(substitutionIdx = 0, 4)
        val leftLeafHash = "H<H<k1v1>H<k2v2>>"
        val midLeafHash = new String(hasher.hash(leafProof.childrenChecksums.flatten))
        val rightLeafHash = "H<H<k5v5>H<k6v6>>"
        val parent1Tree = createBranchProof(hasher.hash(key2, key4), substitutionIdx = 1, leftLeafHash, midLeafHash, rightLeafHash)
        val parent2Tree = createBranchProof(hasher.hash(key6), substitutionIdx = 0, s"H<H<k2k4>$leftLeafHash$midLeafHash$rightLeafHash>", "h<...>")

        val result = WriteResults(key3, val3, merklePath(parent2Tree, parent1Tree, leafProof), null)
        inspector.verifyPut(result, parent2Tree.calcChecksum(hasher, None)) shouldBe true
      }
      "merkle path hash many elements, element was updated" in {
        val leafProof = createLeafProof(substitutionIdx = 0, 3, 4)
        val leftLeafHash = "H<H<k1v1>H<k2v2>>"
        val midLeafHash = new String(hasher.hash(leafProof.childrenChecksums.flatten))
        val rightLeafHash = "H<H<k5v5>H<k6v6>>"
        val parent1Tree = createBranchProof(hasher.hash(key2, key4), substitutionIdx = 1, leftLeafHash, midLeafHash, rightLeafHash)
        val parent2Tree = createBranchProof(hasher.hash(key6), substitutionIdx = 0, s"H<H<k2k4>$leftLeafHash$midLeafHash$rightLeafHash>", "h<...>")

        val result = WriteResults(key3, val3, merklePath(parent2Tree, parent1Tree, leafProof), null)
        inspector.verifyPut(result, parent2Tree.calcChecksum(hasher, None)) shouldBe true
      }

    }
    "return false always" when {
      "merkle root didn't match" in {
        val result = WriteResults(key1, val1, merklePath(createLeafProof(0, 1, 2)), merklePath(createLeafProof(0, 1, 2)))
        inspector.verifyPut(result, "H<wrong root>".getBytes) shouldBe false
      }
    }
  }

}
