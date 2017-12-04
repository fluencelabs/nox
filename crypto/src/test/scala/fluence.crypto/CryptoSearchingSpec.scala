package fluence.crypto

import org.scalatest.{ Matchers, WordSpec }

import scala.collection.Searching.{ Found, InsertionPoint }

class CryptoSearchingSpec extends WordSpec with Matchers {

  "search" should {
    "correct search plainText key in encrypted data" in {

      val crypt: NoOpCrypt[String] = NoOpCrypt.forString

      val plainTextElements = Array("A", "B", "C", "D", "E")
      val encryptedElements = plainTextElements.map(crypt.encrypt)

      import CryptoSearching._
      implicit val decryptFn: (Array[Byte]) ⇒ String = crypt.decrypt

      encryptedElements.binarySearch("B") shouldBe Found(1)
      encryptedElements.binarySearch("D") shouldBe Found(3)
      encryptedElements.binarySearch("E") shouldBe Found(4)

      encryptedElements.binarySearch("0") shouldBe InsertionPoint(0)
      encryptedElements.binarySearch("BB") shouldBe InsertionPoint(2)
      encryptedElements.binarySearch("ZZ") shouldBe InsertionPoint(5)

    }
  }

}
