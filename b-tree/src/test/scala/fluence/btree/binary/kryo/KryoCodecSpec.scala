package fluence.btree.binary.kryo

import org.scalatest.{ Matchers, WordSpec }

class KryoCodecSpec extends WordSpec with Matchers {

  val testBlob = Array("3".getBytes(), "4".getBytes(), "5".getBytes())
  val testClass = TestClass("one", 2, testBlob)

  "encode" should {
    "fail" when {
      "param in registered" in {
        val codec = KryoCodec(Nil)

        assertThrows[Throwable] {
          codec.encode(testClass)
        }
      }
    }
  }

  "encode and decode" should {
    "be inverse functions" in {

      val codec = KryoCodec(Seq(classOf[TestClass], classOf[Array[Byte]], classOf[Array[Array[Byte]]]))

      val result = codec.decode(codec.encode(testClass))

      result shouldBe a[TestClass]
      val r = result.asInstanceOf[TestClass]
      r.str shouldBe "one"
      r.num shouldBe 2
      r.blob should contain theSameElementsAs testBlob

    }
  }

  "encode" should {
    "don't write full class name to binary representation" when {
      "class registered" in {
        val codec = KryoCodec(Seq(classOf[TestClass], classOf[Array[Byte]], classOf[Array[Array[Byte]]]))
        val encoded = new String(codec.encode(testClass))
        val reasonableMaxSize = 20 // bytes
        encoded should not contain "TestClass"
        encoded.length should be < reasonableMaxSize
      }
    }

  }
}

case class TestClass(str: String, num: Long, blob: Array[Array[Byte]])

