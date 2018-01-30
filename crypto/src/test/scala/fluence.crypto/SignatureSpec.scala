package fluence.crypto

import java.security.Security

import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.signature.{ SignatureChecker, Signer }
import org.bouncycastle.jce.provider.BouncyCastleProvider
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.util.Random

class SignatureSpec extends WordSpec with Matchers with BeforeAndAfterAll {
  override protected def beforeAll(): Unit = {
    Security.addProvider(new BouncyCastleProvider())
  }

  override protected def afterAll(): Unit = {
    Security.removeProvider("BC")
  }

  "ecdsa algorithm" should {
    "correct sign and verify data" in {
      val algorithm = Ecdsa.ecdsa_secp256k1_sha256

      val keys = algorithm.generateKeyPair()
      val data = ByteVector(Random.nextString(10).getBytes)

      val sign = algorithm.sign(keys, data)
      algorithm.verify(sign, data) shouldBe true

      val randomData = ByteVector(Random.nextString(10).getBytes)
      val randomSign = algorithm.sign(keys, randomData)
      algorithm.verify(sign.copy(sign = randomSign.sign), data) shouldBe false
    }

    "correctly work with signer and checker" in {
      val keys = Ecdsa.ecdsa_secp256k1_sha256.generateKeyPair()
      val signer = new Signer.EcdsaSigner(keys)

      val data = ByteVector(Random.nextString(10).getBytes)
      val sign = signer.sign(data)

      SignatureChecker.EcdsaChecker.check(sign, data) shouldBe true

      val randomSign = signer.sign(ByteVector(Random.nextString(10).getBytes))
      SignatureChecker.EcdsaChecker.check(randomSign, data) shouldBe false
    }
  }
}
