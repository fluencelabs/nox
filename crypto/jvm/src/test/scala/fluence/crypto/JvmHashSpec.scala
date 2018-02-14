package fluence.crypto

import fluence.crypto.hash.JdkCryptoHasher
import org.scalatest.{ Matchers, WordSpec }
import scodec.bits.ByteVector

class JvmHashSpec extends WordSpec with Matchers {
  "js hasher" should {
    //test values get from third-party hash services
    "work with sha256" in {
      val str = "sha256Tester"
      val sha256TesterHex = "513c17f8cf6ba96ce412cc2ae82f68821e9a2c6ae7a2fb1f5e46d08c387c8e65"

      val hasher = JdkCryptoHasher.Sha256
      ByteVector(hasher.hash(str.getBytes())).toHex shouldBe sha256TesterHex
    }

    "work with sha1" in {
      val str = "sha1Tester"
      val sha1TesterHex = "879db20eabcecea7d4736a8bae5bc64564b76b2f"

      val hasher = JdkCryptoHasher.Sha1
      ByteVector(hasher.hash(str.getBytes())).toHex shouldBe sha1TesterHex
    }
  }
}
