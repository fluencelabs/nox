package fluence.crypto

import fluence.crypto.algorithm.{ AesCrypt, CryptoErr }
import cats.instances.try_._
import org.scalatest.{ Matchers, WordSpec }

import scala.util.{ Random, Try }

class AesSpec extends WordSpec with Matchers {

  def rndString(size: Int): String = Random.nextString(10)

  "aes crypto" should {
    "work with IV" in {

      val pass = "pass".toCharArray
      val crypt = AesCrypt.forString[Try](pass, withIV = true)

      val str = rndString(200)
      val crypted = crypt.encrypt(str).get
      crypt.decrypt(crypted).get shouldBe str

      val fakeAes = AesCrypt.forString[Try]("wrong".toCharArray, withIV = true)
      fakeAes.decrypt(crypted).map(_ ⇒ false).recover {
        case e: CryptoErr ⇒ true
        case _            ⇒ false
      }.get shouldBe true

      //we cannot check if first bytes is iv or already data, but encryption goes wrong
      val aesWithoutIV = AesCrypt.forString[Try](pass, withIV = false)
      aesWithoutIV.decrypt(crypted).get shouldNot be (str)

      val aesWrongSalt = AesCrypt.forString[Try](pass, withIV = true, rndString(10).getBytes())
      aesWrongSalt.decrypt(crypted).map(_ ⇒ false).recover {
        case e: CryptoErr ⇒ true
        case _            ⇒ false
      }.get shouldBe true
    }

    "work without IV" in {
      val pass = "pass".toCharArray
      val crypt = AesCrypt.forString[Try](pass, withIV = false)

      val str = rndString(200)
      val crypted = crypt.encrypt(str).get
      crypt.decrypt(crypted).get shouldBe str

      val fakeAes = AesCrypt.forString[Try]("wrong".toCharArray, withIV = false)
      fakeAes.decrypt(crypted).map(_ ⇒ false).recover {
        case e: CryptoErr ⇒ true
        case _            ⇒ false
      }.get shouldBe true

      //we cannot check if first bytes is iv or already data, but encryption goes wrong
      val aesWithIV = AesCrypt.forString[Try](pass, withIV = true)
      aesWithIV.decrypt(crypted).get shouldNot be (str)

      val aesWrongSalt = AesCrypt.forString[Try](pass, withIV = true, rndString(10).getBytes())
      aesWrongSalt.decrypt(crypted).map(_ ⇒ false).recover {
        case e: CryptoErr ⇒ true
        case _            ⇒ false
      }.get shouldBe true
    }
  }

}
