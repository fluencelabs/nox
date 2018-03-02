package fluence.crypto

import fluence.crypto.algorithm.{ AesConfig, AesCrypt, CryptoErr }
import cats.instances.try_._
import org.scalactic.source.Position
import org.scalatest.{ Assertion, Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.util.{ Random, Try }

class AesSpec extends WordSpec with Matchers with slogging.LazyLogging {

  def rndString(size: Int): String = Random.nextString(10)
  val conf = AesConfig()

  "aes crypto" should {
    "work with IV" in {

      val pass = ByteVector("pass".getBytes())
      val crypt = AesCrypt.forString[Try](pass, withIV = true, config = conf)

      val str = rndString(200)
      println("encrypt")
      val crypted = crypt.encrypt(str).get
      println("decrypt")
      crypt.decrypt(crypted).get shouldBe str
      println("decrypted")
      val fakeAes = AesCrypt.forString[Try](ByteVector("wrong".getBytes()), withIV = true, config = conf)
      checkCryptoError(fakeAes.decrypt(crypted))

      //we cannot check if first bytes is iv or already data, but encryption goes wrong
      val aesWithoutIV = AesCrypt.forString[Try](pass, withIV = false, config = conf)
      aesWithoutIV.decrypt(crypted).get shouldNot be (str)

      val aesWrongSalt = AesCrypt.forString[Try](pass, withIV = true, config = conf.copy(salt = rndString(10)))
      checkCryptoError(aesWrongSalt.decrypt(crypted))
    }

    "work without IV" in {
      val pass = ByteVector("pass".getBytes())
      val crypt = AesCrypt.forString[Try](pass, withIV = false, config = conf)

      val str = rndString(200)
      val crypted = crypt.encrypt(str).get
      crypt.decrypt(crypted).get shouldBe str

      val fakeAes = AesCrypt.forString[Try](ByteVector("wrong".getBytes()), withIV = false, config = conf)
      checkCryptoError(fakeAes.decrypt(crypted))

      //we cannot check if first bytes is iv or already data, but encryption goes wrong
      val aesWithIV = AesCrypt.forString[Try](pass, withIV = true, config = conf)
      aesWithIV.decrypt(crypted).get shouldNot be (str)

      val aesWrongSalt = AesCrypt.forString[Try](pass, withIV = true, config = conf.copy(salt = rndString(10)))
      checkCryptoError(aesWrongSalt.decrypt(crypted))
    }
  }

  def checkCryptoError(tr: Try[String])(implicit pos: Position): Assertion = {
    tr.map(_ ⇒ false).recover {
      case e: CryptoErr ⇒ true
      case e ⇒
        logger.error("Unexpected error", e)
        false
    }.get shouldBe true
  }

}
