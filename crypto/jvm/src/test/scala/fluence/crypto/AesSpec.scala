package fluence.crypto

import fluence.crypto.algorithm.{ AesConfig, AesCrypt, CryptoErr }
import cats.instances.try_._
import org.scalatest.{ Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.util.{ Random, Try }

class AesSpec extends WordSpec with Matchers {

  def rndString(size: Int): String = Random.nextString(10)
  val conf = AesConfig()

  "aes crypto" should {
    "work with IV" in {

      val pass = ByteVector("pass".getBytes())
      val crypt = AesCrypt.forString[Try](pass, withIV = true, config = conf)

      val str = rndString(200)
      val crypted = crypt.encrypt(str).get
      crypt.decrypt(crypted).get shouldBe str

      val fakeAes = AesCrypt.forString[Try](ByteVector("wrong".getBytes()), withIV = true, config = conf)
      fakeAes.decrypt(crypted).map(_ ⇒ false).recover {
        case e: CryptoErr ⇒ true
        case _            ⇒ false
      }.get shouldBe true

      //we cannot check if first bytes is iv or already data, but encryption goes wrong
      val aesWithoutIV = AesCrypt.forString[Try](pass, withIV = false, config = conf)
      aesWithoutIV.decrypt(crypted).get shouldNot be (str)

      val aesWrongSalt = AesCrypt.forString[Try](pass, withIV = true, config = conf.copy(salt = rndString(10)))
      aesWrongSalt.decrypt(crypted).map(_ ⇒ false).recover {
        case e: CryptoErr ⇒ true
        case _            ⇒ false
      }.get shouldBe true
    }

    "work without IV" in {
      val pass = ByteVector("pass".getBytes())
      val crypt = AesCrypt.forString[Try](pass, withIV = false, config = conf)

      val str = rndString(200)
      val crypted = crypt.encrypt(str).get
      crypt.decrypt(crypted).get shouldBe str

      val fakeAes = AesCrypt.forString[Try](ByteVector("wrong".getBytes()), withIV = false, config = conf)
      fakeAes.decrypt(crypted).map(_ ⇒ false).recover {
        case e: CryptoErr ⇒ true
        case _            ⇒ false
      }.get shouldBe true

      //we cannot check if first bytes is iv or already data, but encryption goes wrong
      val aesWithIV = AesCrypt.forString[Try](pass, withIV = true, config = conf)
      aesWithIV.decrypt(crypted).get shouldNot be (str)

      val aesWrongSalt = AesCrypt.forString[Try](pass, withIV = true, config = conf.copy(salt = rndString(10)))
      aesWrongSalt.decrypt(crypted).map(_ ⇒ false).recover {
        case e: CryptoErr ⇒ true
        case _            ⇒ false
      }.get shouldBe true
    }
  }

}
