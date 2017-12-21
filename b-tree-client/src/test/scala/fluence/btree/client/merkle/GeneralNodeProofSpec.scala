package fluence.btree.client.merkle

import fluence.hash.TestCryptoHasher
import org.scalatest.{ Matchers, WordSpec }

class GeneralNodeProofSpec extends WordSpec with Matchers {

  private val testHasher = TestCryptoHasher

  "calcChecksum" should {
    "calculate correct checksum" when {
      "stateChecksum is empty, childsChecksums is empty, substitution checksum is None" in {
        val result = GeneralNodeProof(Array.emptyByteArray, Array.empty, 1)
          .calcChecksum(testHasher, None)
        result shouldBe Array.emptyByteArray
      }
      "stateChecksum is empty, substitution checksum is None" in {
        val result = GeneralNodeProof(Array.emptyByteArray, Array("A".getBytes, "B".getBytes, "C".getBytes), 1)
          .calcChecksum(testHasher, None)
        new String(result) shouldBe "H<ABC>"
      }
      "stateChecksum is empty, substitution checksum is defined" in {
        val result = GeneralNodeProof(Array.emptyByteArray, Array("A".getBytes, "B".getBytes, "C".getBytes), 1)
          .calcChecksum(testHasher, Some("X".getBytes))
        new String(result) shouldBe "H<AXC>"
      }
      "stateChecksum is defined, substitution checksum is None" in {
        val result = GeneralNodeProof("STATE_".getBytes, Array("A".getBytes, "B".getBytes, "C".getBytes), 1)
          .calcChecksum(testHasher, None)
        new String(result) shouldBe "H<STATE_ABC>"
      }
      "stateChecksum is defined, substitution checksum is defined" in {
        val result = GeneralNodeProof("STATE_".getBytes, Array("A".getBytes, "B".getBytes, "C".getBytes), 1)
          .calcChecksum(testHasher, Some("X".getBytes))
        new String(result) shouldBe "H<STATE_AXC>"
      }
    }
    "throw exception" when {
      "childrenChecksums is empty" in {
        intercept[ArrayIndexOutOfBoundsException] {
          GeneralNodeProof("STATE_".getBytes, Array.empty[Array[Byte]], 0)
            .calcChecksum(testHasher, Some("X".getBytes))
        }
      }
      "substitution index out of bound" in {
        intercept[ArrayIndexOutOfBoundsException] {
          GeneralNodeProof("STATE_".getBytes, Array("A".getBytes, "B".getBytes, "C".getBytes), 10)
            .calcChecksum(testHasher, Some("X".getBytes))

        }
      }
    }
  }

}
