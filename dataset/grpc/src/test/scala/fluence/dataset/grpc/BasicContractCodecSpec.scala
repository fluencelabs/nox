package fluence.dataset.grpc

import cats.instances.try_._
import cats.kernel.Eq
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature
import fluence.crypto.signature.Signer
import org.scalatest.{ Matchers, WordSpec }
import fluence.dataset.{ BasicContract ⇒ BC }
import fluence.kad.protocol.Key

import scala.util.{ Success, Try }

class BasicContractCodecSpec extends WordSpec with Matchers {

  "BasicContractCodec" should {
    def checkInvariance(bc: BC) = {
      val codec = BasicContractCodec.codec[Try]

      val Success(mod) = (codec.direct andThen codec.inverse).run(bc)

      Eq.eqv(mod.id, bc.id) shouldBe true
      mod.offer shouldBe bc.offer
      mod.offerSeal.publicKey.value shouldBe bc.offerSeal.publicKey.value
      mod.offerSeal.sign shouldBe bc.offerSeal.sign
      mod.participants.keySet should contain theSameElementsAs bc.participants.keySet

      mod shouldBe bc
    }

    "be invariant for direct+inverse op" in {

      val seed = "seed".getBytes()
      val keyPair = KeyPair.fromBytes(seed, seed)
      val signer = new signature.Signer.DumbSigner(keyPair)
      val key = Key.fromKeyPair(keyPair).get

      Seq(
        BC.offer(key, 1, signer)
      ).foreach(checkInvariance)

    }
  }

}
