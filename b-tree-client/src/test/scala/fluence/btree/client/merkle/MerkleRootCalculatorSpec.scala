package fluence.btree.client.merkle

import fluence.hash.TestCryptoHasher
import org.scalatest.{ Matchers, WordSpec }

class MerkleRootCalculatorSpec extends WordSpec with Matchers {

  private val hash1 = "1".getBytes()
  private val hash2 = "2".getBytes()
  private val hash3 = "3".getBytes()
  private val hashX = "X".getBytes()

  private val proof = MerkleRootCalculator(TestCryptoHasher)

  "calcMerkleRoot without hash" should {
    "calculate valid root" when {
      "merkle path has one element with one hash" in {
        val result = proof.calcMerkleRoot(MerklePath(Array(NodeProof(Array(hash1), 0))))

        new String(result) shouldBe "H<1>"
      }
      "merkle path has one element with many hashes" in {
        val result = proof.calcMerkleRoot(MerklePath(Array(NodeProof(Array(hash1, hash2, hash3), 0))))

        new String(result) shouldBe "H<123>"
      }
      "merkle path has many elements with many hashes" in {
        val node1 = NodeProof(Array(hash1, hash1), 0)
        val node2 = NodeProof(Array(hash2, hash2), 1)
        val node3 = NodeProof(Array(hash3, hash3, hash3), 1)
        val result = proof.calcMerkleRoot(MerklePath(Array(node3, node2, node1)))

        new String(result) shouldBe "H<3H<2H<11>>3>"
      }
    }

    "returns empty byte array" when {
      "merkle path is empty" in {
        val result = proof.calcMerkleRoot(MerklePath(Array.empty[NodeProof]))
        result shouldBe Array.emptyByteArray
      }
    }
  }

  "calcMerkleRoot with hash" should {
    "substitutes hash and calculate valid root" when {
      "merkle path has one element with one hash" in {
        val result = proof.calcMerkleRoot(MerklePath(Array(NodeProof(Array(hash1), 0))), hashX)

        new String(result) shouldBe "H<X>"
      }
      "merkle path has one element with many hashes" in {
        val result = proof.calcMerkleRoot(MerklePath(Array(NodeProof(Array(hash1, hash2, hash3), 1))), hashX)

        new String(result) shouldBe "H<1X3>"
      }
      "merkle path has many elements with many hashes" in {
        val node1 = NodeProof(Array(hash1, hash1), 0)
        val node2 = NodeProof(Array(hash2, hash2), 1)
        val node3 = NodeProof(Array(hash3, hash3, hash3), 1)
        val result = proof.calcMerkleRoot(MerklePath(Array(node3, node2, node1)), hashX)

        new String(result) shouldBe "H<3H<2H<X1>>3>"
      }
    }
  }
}
