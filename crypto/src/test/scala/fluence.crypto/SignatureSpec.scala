package fluence.crypto

import java.security.Security

import cats.syntax.all._
import cats.instances.try_._
import cats.{ Id, MonadError }
import fluence.crypto.algorithm.{ CryptoErr, Ecdsa }
import fluence.crypto.signature.{ SignatureChecker, Signer }
import org.bouncycastle.jce.provider.BouncyCastleProvider
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.util.{ Random, Try }

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

      val t = for {
        keys ← algorithm.generateKeyPair[Try]()
        data = ByteVector(Random.nextString(10).getBytes)
        sign ← algorithm.sign(keys, data)
        goodRes ← algorithm.verify(sign, data)
        _ = goodRes shouldBe true
        randomData = ByteVector(Random.nextString(10).getBytes)
        randomSign = algorithm.sign(keys, randomData)
        badRes ← algorithm.verify(sign.copy(sign = randomSign.get.sign), data)
        _ = badRes shouldBe false
      } yield {}
      t.get
    }

    "correctly work with signer and checker" in {
      val t = for {
        keys ← Ecdsa.ecdsa_secp256k1_sha256.generateKeyPair()
        signer = new Signer.EcdsaSigner(keys)

        data = ByteVector(Random.nextString(10).getBytes)
        sign ← signer.sign(data)

        goodCheck ← SignatureChecker.EcdsaChecker.check(sign, data)
        _ = goodCheck shouldBe true

        randomSign ← signer.sign(ByteVector(Random.nextString(10).getBytes))
        badCheck ← SignatureChecker.EcdsaChecker.check(randomSign, data)
        _ = badCheck shouldBe false
      } yield {}
      t.get
    }
  }
}
