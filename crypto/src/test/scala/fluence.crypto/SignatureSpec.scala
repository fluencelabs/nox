/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.crypto

import cats.instances.try_._
import fluence.crypto.algorithm.{ CryptoErr, Ecdsa }
import fluence.crypto.keypair.KeyPair
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.util.{ Random, Try }

class SignatureSpec extends WordSpec with Matchers with BeforeAndAfterAll {

  def rndByteVector(size: Int) = ByteVector(Random.nextString(10).getBytes)

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
      val keys = Ecdsa.ecdsa_secp256k1_sha256[Try].generateKeyPair().get
      val signer = new Ecdsa.Signer(keys)
      val data = rndByteVector(10)

      val brokenSecret = keys.copy(secretKey = KeyPair.Secret(rndByteVector(10)))
      val brokenSigner = new Ecdsa.Signer(brokenSecret)

      the[CryptoErr] thrownBy brokenSigner.sign(data).get

      val sign = signer.sign(data).get

      the[CryptoErr] thrownBy Ecdsa.Checker.check(sign.copy(sign = rndByteVector(10)), data).get
      the[CryptoErr] thrownBy Ecdsa.Checker.check(sign.copy(publicKey = sign.publicKey.copy(value = rndByteVector(10))), data).get
    }
  }
}
