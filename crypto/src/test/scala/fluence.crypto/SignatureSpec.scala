package fluence.crypto

import java.security.Security

import cats.syntax.all._
import cats.{Id, MonadError}
import fluence.crypto.algorithm.{CryptoErr, Ecdsa}
import org.bouncycastle.jce.provider.BouncyCastleProvider
import org.scalatest.{BeforeAndAfterAll, Matchers, WordSpec}
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

      implicit val ME = new MonadError[Id, CryptoErr] {
        override def raiseError[A](e: CryptoErr): Id[A] = ???
        override def handleErrorWith[A](fa: Id[A])(f: CryptoErr ⇒ Id[A]): Id[A] = ???
        override def flatMap[A, B](fa: Id[A])(f: A ⇒ Id[B]): Id[B] = fa.flatMap(f)
        override def tailRecM[A, B](a: A)(f: A ⇒ Id[Either[A, B]]): Id[B] = ???
        override def pure[A](x: A): Id[A] = x
      }

      for {
        keys ← algorithm.generateKeyPair[Id]()
        data = ByteVector(Random.nextString(10).getBytes)
        sign ← algorithm.sign(keys, data)
      } yield {
        algorithm.verify(sign, data) shouldBe true

        val randomData = ByteVector(Random.nextString(10).getBytes)
        val randomSign = algorithm.sign(keys, randomData)
        algorithm.verify(sign.copy(sign = randomSign.sign), data) shouldBe false
      }
    }

    /*"correctly work with signer and checker" in {
      val keys = Ecdsa.ecdsa_secp256k1_sha256.generateKeyPair()
      val signer = new Signer.EcdsaSigner(keys)

      val data = ByteVector(Random.nextString(10).getBytes)
      val sign = signer.sign(data)

      SignatureChecker.EcdsaChecker.check(sign, data) shouldBe true

      val randomSign = signer.sign(ByteVector(Random.nextString(10).getBytes))
      SignatureChecker.EcdsaChecker.check(randomSign, data) shouldBe false
    }*/
  }
}
