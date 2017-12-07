package fluence.btree.client.common

import org.scalatest.{ Matchers, WordSpec }

class BytesOpsSpec extends WordSpec with Matchers {

  "copyOf" should {
    "correct copy byte array" when {
      "array is empty" in {
        BytesOps.copyOf(Array.emptyByteArray) shouldBe Array.emptyByteArray
      }
      "array if filled" in {
        val source = "source".getBytes
        val copy = BytesOps.copyOf(source)
        copy eq source shouldBe false
        copy shouldBe source
      }
    }
    "correct copy array of byte array" when {
      "array is empty" in {
        BytesOps.copyOf(Array.empty[Array[Byte]]) shouldBe Array.empty[Array[Byte]]
      }
      "array if filled" in {
        val source = Array("A".getBytes, "B".getBytes, "C".getBytes)
        val copy = BytesOps.copyOf(source)
        copy eq source shouldBe false
        copy shouldBe source
      }
    }
  }

  "rewriteValue" should {
    "throw exception" when {
      "source array is empty" in {
        intercept[ArrayIndexOutOfBoundsException] {
          BytesOps.rewriteValue(Array.empty[Array[Byte]], "A".getBytes, 0) shouldBe Array.empty[Array[Byte]]
        }
      }
      "idx out of bound" in {
        intercept[ArrayIndexOutOfBoundsException] {
          BytesOps.rewriteValue(Array("A".getBytes), "A".getBytes, 10) shouldBe Array.empty[Array[Byte]]
        }
      }
    }

    "copy source array and update value" when {
      "array if filled" in {
        val source = Array("A".getBytes, "B".getBytes, "C".getBytes)
        val updatedSource = BytesOps.rewriteValue(source, "X".getBytes, 1)
        updatedSource eq source shouldBe false
        updatedSource should contain theSameElementsInOrderAs Array("A".getBytes, "X".getBytes, "C".getBytes)
      }
    }
  }

}
