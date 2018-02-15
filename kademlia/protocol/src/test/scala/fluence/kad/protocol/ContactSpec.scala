package fluence.kad.protocol

import java.net.InetAddress

import cats._
import fluence.crypto.algorithm.Ecdsa
import org.scalatest.{ Matchers, WordSpec }

class ContactSpec extends WordSpec with Matchers {

  "Contact" should {
    "serialize and deserialize in Id" in {

      val algo = Ecdsa.signAlgo

      val Right(kp) = algo.generateKeyPair[Id]().value

      val c = Contact.buildOwn[Id](
        InetAddress.getLocalHost,
        8080,
        kp.publicKey,
        10l,
        "hash",
        algo.signer(kp)
      ).value.right.get

      val seed = c.b64seed

      Contact.readB64seed[Id](seed, algo.checker).value.isRight shouldBe true
      Contact.readB64seed[Id](seed, algo.checker).value shouldBe Right(c)
    }
  }

}
