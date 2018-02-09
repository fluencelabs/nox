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

import java.io.File
import java.security.SecureRandom

import cats.instances.try_._
import fluence.crypto.algorithm.{ CryptoErr, Ecdsa }
import fluence.crypto.keypair.KeyPair
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.util.Random

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

      import fluence.crypto.KeyStore._
      import io.circe.parser.decode
      import io.circe.syntax._
      import io.circe.{ Decoder, Encoder, HCursor, Json }

      val json =
        """
          |{
          |  "keystore" : {
          |    "secret" : "wTnTLZf38K1JfyW3ue8vEtT/DmqLUse7YCDL+ksn48M=",
          |    "public" : "A8t173PRrzU1u805NKNLsijCwvkTi0cJge0hLgwPQj5A"
          |  }
          |}
        """.stripMargin

      val keyPair = decode[Option[KeyStore]](json).right.get.get.keyPair

      val sign1 = algorithm.sign(keyPair, data).get
      val test = algorithm.verify(sign1, data).get
      println("TEST ==== " + test)
    }

    "correctly work with signer and checker" in {
      val algo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)
      val keys = algo.generateKeyPair().get
      val signer = algo.signer(keys)

      val data = rndByteVector(10)
      val sign = signer.sign(data).get

      algo.checker.check(sign, data).get shouldBe true

      val randomSign = signer.sign(rndByteVector(10)).get
      algo.checker.check(randomSign, data).get shouldBe false
    }

    "throw an errors on invalid data" in {
      val algo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)
      val keys = algo.generateKeyPair().get
      val signer = algo.signer(keys)
      val data = rndByteVector(10)

      val sign = signer.sign(data).get

      the[CryptoErr] thrownBy algo.checker.check(sign.copy(sign = rndByteVector(10)), data).get
      the[CryptoErr] thrownBy algo.checker.check(sign.copy(publicKey = sign.publicKey.copy(value = rndByteVector(10))), data).get
    }

    "store and read key from file" in {

      println("STORING ===========")
      val algo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)
      val keys = algo.generateKeyPair().get

      val keyFile = File.createTempFile("test", "")
      if (keyFile.exists()) keyFile.delete()
      val storage = new FileKeyStorage(keyFile)

      storage.storeSecretKey(keys)

      val keysReadE = storage.readKeyPair
      val keysRead = keysReadE.get

      val signer = algo.signer(keys)
      val data = rndByteVector(10)
      val sign = signer.sign(data).get

      algo.checker.check(sign.copy(publicKey = keysRead.publicKey), data).get shouldBe true
      algo.checker.check(sign, data).get shouldBe true

      //try to store key into previously created file
      storage.storeSecretKey(keys).isFailure shouldBe true
    }
  }
}
