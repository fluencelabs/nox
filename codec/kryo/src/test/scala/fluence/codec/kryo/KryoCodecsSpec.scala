package fluence.codec.kryo

import cats.instances.try_._
import org.scalatest.{ Matchers, WordSpec }

import scala.util.Try

class KryoCodecsSpec extends WordSpec with Matchers {

  val testBlob = Array("3".getBytes(), "4".getBytes(), "5".getBytes())
  val testClass = TestClass("one", 2, testBlob)

  private val testCodecs =
    KryoCodecs()
      .add[Array[Array[Byte]]]
      .addCase(classOf[TestClass]).build[Try]()

  "encode and decode" should {
    "be inverse functions" when {
      "object defined" in {

        val codec = testCodecs.codec[TestClass]

        val result = codec.encode(testClass).flatMap(codec.decode).get

        result.str shouldBe "one"
        result.num shouldBe 2
        result.blob should contain theSameElementsAs testBlob
      }

      "object is null" in {
        val codec = testCodecs.codec[TestClass]
        val result = codec.encode(null).flatMap(codec.decode)
        result.isFailure shouldBe true
      }
    }
  }

  "encode" should {
    "don't write full class name to binary representation" when {
      "class registered" in {
        //val codec = KryoCodec[TestClass](Seq(classOf[TestClass], classOf[Array[Byte]], classOf[Array[Array[Byte]]]), registerRequired = true)
        val codec = testCodecs.codec[TestClass]
        val encoded = codec.encode(testClass).map(new String(_)).get
        val reasonableMaxSize = 20 // bytes
        encoded should not contain "TestClass"
        encoded.length should be < reasonableMaxSize
      }
    }

  }
}

case class TestClass(str: String, num: Long, blob: Array[Array[Byte]])

