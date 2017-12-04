package fluence.crypto

import org.scalatest.{ Matchers, WordSpec }

class NoOpCryptSpec extends WordSpec with Matchers {

  "NoOpCrypt" should {
    "convert a string to bytes back and forth without any cryptography" in {

      val noOpCrypt = NoOpCrypt.forString

      val emptyString = ""
      noOpCrypt.decrypt(noOpCrypt.encrypt(emptyString)) shouldBe emptyString
      val nonEmptyString = "some text here"
      noOpCrypt.decrypt(noOpCrypt.encrypt(nonEmptyString)) shouldBe nonEmptyString
      val byteArray = Array(1.toByte, 23.toByte, 45.toByte)
      noOpCrypt.encrypt(noOpCrypt.decrypt(byteArray)) shouldBe byteArray
    }

    "convert a long to bytes back and forth without any cryptography" in {

      val noOpCrypt = NoOpCrypt.forLong

      noOpCrypt.decrypt(noOpCrypt.encrypt(0L)) shouldBe 0L
      noOpCrypt.decrypt(noOpCrypt.encrypt(1234567890123456789L)) shouldBe 1234567890123456789L
    }
  }
}
