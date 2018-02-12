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

import cats.data.EitherT
import cats.instances.try_._
import fluence.crypto.algorithm.{ CryptoErr, Ecdsa }
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.util.{ Random, Try }

class SignatureSpec extends WordSpec with Matchers with BeforeAndAfterAll {

  def rndBytes(size: Int) = Random.nextString(10).getBytes

  def rndByteVector(size: Int) = ByteVector(rndBytes(size))

  private implicit class TryEitherTExtractor[A, B](et: EitherT[Try, A, B]) {
    def extract: B = et.value.get.right.get
  }

  "ecdsa algorithm" should {
    "correct sign and verify data" in {
      val algorithm = Ecdsa.ecdsa_secp256k1_sha256

      val keys = algorithm.generateKeyPair[Try]().extract
      val data = rndByteVector(10)
      val sign = algorithm.sign[Try](keys, data).extract

      algorithm.verify[Try](sign, data).value.get.isRight shouldBe true

      val randomData = rndByteVector(10)
      val randomSign = algorithm.sign(keys, randomData).extract

      algorithm.verify(sign.copy(sign = randomSign.sign), data).extract shouldBe false

      algorithm.verify(sign, randomData).extract shouldBe false
    }

    "correctly work with signer and checker" in {
      val algo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)
      val keys = algo.generateKeyPair().extract
      val signer = algo.signer(keys)

      val data = rndByteVector(10)
      val sign = signer.sign(data).extract

      algo.checker.check(sign, data).value.get.isRight shouldBe true

      val randomSign = signer.sign(rndByteVector(10)).extract
      algo.checker.check(randomSign, data).value.get.isRight shouldBe false
    }

    "throw an errors on invalid data" in {
      val algo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)
      val keys = algo.generateKeyPair().extract
      val signer = algo.signer(keys)
      val data = rndByteVector(10)

      val sign = signer.sign(data).extract

      the[CryptoErr] thrownBy algo.checker.check(sign.copy(sign = rndByteVector(10)), data).extract
      the[CryptoErr] thrownBy algo.checker.check(sign.copy(publicKey = sign.publicKey.copy(value = rndByteVector(10))), data).extract
    }

    "store and read key from file" in {
      val algo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)
      val keys = algo.generateKeyPair().extract

      val keyFile = File.createTempFile("test", "")
      if (keyFile.exists()) keyFile.delete()
      val storage = new FileKeyStorage(keyFile)

      storage.storeSecretKey(keys)

      val keysReadE = storage.readKeyPair
      val keysRead = keysReadE.get

      val signer = algo.signer(keys)
      val data = rndByteVector(10)
      val sign = signer.sign(data).extract

      algo.checker.check(sign.copy(publicKey = keysRead.publicKey), data).extract shouldBe true
      algo.checker.check(sign, data).value.get.isRight shouldBe true

      //try to store key into previously created file
      storage.storeSecretKey(keys).isFailure shouldBe true
    }
  }
}
