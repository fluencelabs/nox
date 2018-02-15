package fluence.kad.protocol

import java.net.InetAddress

import org.scalatest.{ Matchers, WordSpec }
import cats._
import cats.data.Ior
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa

class ContactSpec extends WordSpec with Matchers {

  "Contact" should {
    "serialize and deserialize in Id" in {

      val algo = Ecdsa.signAlgo

      val Right(kp) = algo.generateKeyPair[Id]().value

      val c = Contact(
        InetAddress.getLocalHost,
        8080,
        kp.publicKey,
        10l,
        "hash",
        Ior.left(algo.signer(kp))
      )

      val Right(seed) = c.b64seed[Id].value

      Contact.readB64seed[Id](seed, algo.checker).value.isRight shouldBe true

    }
  }

}
