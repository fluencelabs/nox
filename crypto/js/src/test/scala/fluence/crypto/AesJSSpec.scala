package fluence.crypto

import cats.instances.try_._
import fluence.crypto.algorithm.{ AesConfig, AesCrypt, CryptoErr }
import org.scalactic.source.Position
import org.scalatest.{ Assertion, Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.util.{ Random, Try }

class AesJSSpec extends WordSpec with Matchers with slogging.LazyLogging {

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
      checkCryptoError(fakeAes.decrypt(crypted), str)

      //we cannot check if first bytes is iv or already data, but encryption goes wrong
      val aesWithoutIV = AesCrypt.forString[Try](pass, withIV = false, config = conf)
      aesWithoutIV.decrypt(crypted).get shouldNot be (str)

      val aesWrongSalt = AesCrypt.forString[Try](pass, withIV = true, config = conf.copy(salt = rndString(10)))
      checkCryptoError(aesWrongSalt.decrypt(crypted), str)
    }

    "work without IV" in {
      val pass = ByteVector("pass".getBytes())
      val crypt = AesCrypt.forString[Try](pass, withIV = false, config = conf)

      val str = rndString(200)
      val crypted = crypt.encrypt(str).get
      crypt.decrypt(crypted).get shouldBe str

      val fakeAes = AesCrypt.forString[Try](ByteVector("wrong".getBytes()), withIV = false, config = conf)
      checkCryptoError(fakeAes.decrypt(crypted), str)

      //we cannot check if first bytes is iv or already data, but encryption goes wrong
      val aesWithIV = AesCrypt.forString[Try](pass, withIV = true, config = conf)
      aesWithIV.decrypt(crypted).get shouldNot be (str)

      val aesWrongSalt = AesCrypt.forString[Try](pass, withIV = false, config = conf.copy(salt = rndString(10)))
      checkCryptoError(aesWrongSalt.decrypt(crypted), str)
    }

    def checkCryptoError(tr: Try[String], msg: String)(implicit pos: Position): Assertion = {
      tr.map{ r ⇒ r != msg }.recover {
        case e: CryptoErr ⇒ true
        case e ⇒
          logger.error("Unexpected error", e)
          false
      }.get shouldBe true
    }
  }
}
