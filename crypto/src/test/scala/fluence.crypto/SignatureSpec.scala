package fluence.crypto

import java.io.File
import java.security.SecureRandom

import cats.instances.try_._
import fluence.crypto.algorithm.{ CryptoErr, Ecdsa }
import fluence.crypto.keypair.KeyPair
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.util.{ Random, Try }

class SignatureSpec extends WordSpec with Matchers with BeforeAndAfterAll {

  def rndBytes(size: Int) = Random.nextString(10).getBytes
  def rndByteVector(size: Int) = ByteVector(rndBytes(size))

  "ecdsa algorithm" should {
    "correct sign and verify data" in {
      val algorithm = Ecdsa.ecdsa_secp256k1_sha256

      val keys = algorithm.generateKeyPair().get
      val data = rndByteVector(10)
      val sign = algorithm.sign(keys, data).get

      algorithm.verify(sign, data).get shouldBe true

      val randomData = rndByteVector(10)
      val randomSign = algorithm.sign(keys, randomData).get

      algorithm.verify(sign.copy(sign = randomSign.sign), data).get shouldBe false

      algorithm.verify(sign, randomData).get shouldBe false
    }

    "correctly work with signer and checker" in {
      val keys = Ecdsa.ecdsa_secp256k1_sha256.generateKeyPair().get
      val signer = new Ecdsa.Signer(keys)

      val data = rndByteVector(10)
      val sign = signer.sign(data).get

      Ecdsa.Checker.check(sign, data).get shouldBe true

      val randomSign = signer.sign(rndByteVector(10)).get
      Ecdsa.Checker.check(randomSign, data).get shouldBe false
    }

    "throw an errors on invalid data" in {
      val keys = Ecdsa.ecdsa_secp256k1_sha256.generateKeyPair().get
      val signer = new Ecdsa.Signer(keys)
      val data = rndByteVector(10)

      val brokenSecret = keys.copy(secretKey = KeyPair.Secret(rndByteVector(10)))
      val brokenSigner = new Ecdsa.Signer(brokenSecret)

      the[CryptoErr] thrownBy brokenSigner.sign(data).get

      val sign = signer.sign(data).get

      the[CryptoErr] thrownBy Ecdsa.Checker.check(sign.copy(sign = rndByteVector(10)), data).get
      the[CryptoErr] thrownBy Ecdsa.Checker.check(sign.copy(publicKey = sign.publicKey.copy(value = rndByteVector(10))), data).get
    }

    "store and read key from file" in {
      val seed = rndBytes(32)
      val keys = Ecdsa.ecdsa_secp256k1_sha256.generateKeyPair(new SecureRandom(seed)).get

      val storage = new FileKeyStorage(new File("/tmp/key"))

      storage.storeSecretKey(keys.secretKey)

      val keysReadE = storage.readKeyPair
      val keysRead = keysReadE.get

      val signer = new Ecdsa.Signer(keys)
      val data = rndByteVector(10)
      val sign = signer.sign(data).get

      Ecdsa.Checker.check(sign.copy(publicKey = keysRead.publicKey), data).get shouldBe true
      Ecdsa.Checker.check(sign, data).get shouldBe true
    }
  }
}
