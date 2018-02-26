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

import cats.data.EitherT
import cats.instances.try_._
import fluence.crypto.algorithm.{ CryptoErr, EcdsaJS }
import org.scalatest.{ Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.util.{ Random, Try }

class EcdsaJSSpec extends WordSpec with Matchers {

  def rndBytes(size: Int) = Random.nextString(10).getBytes

  def rndByteVector(size: Int) = ByteVector(rndBytes(size))

  private implicit class TryEitherTExtractor[A <: Throwable, B](et: EitherT[Try, A, B]) {
    def extract: B = et.value.map {
      case Left(e)  ⇒ fail(e) // for making test fail message more describable
      case Right(v) ⇒ v
    }.get

    def isOk: Boolean = et.value.fold(_ ⇒ false, _.isRight)
  }

  "ecdsa algorithm" should {
    "correct sign and verify data" in {
      val algorithm = EcdsaJS.ecdsa_secp256k1_sha256

      val keys = algorithm.generateKeyPair[Try]().extract
      val data = rndByteVector(10)
      val sign = algorithm.sign[Try](keys, data).extract

      algorithm.verify[Try](sign, data).isOk shouldBe true

      val randomData = rndByteVector(10)
      val randomSign = algorithm.sign(keys, randomData).extract

      algorithm.verify(sign.copy(sign = randomSign.sign), data).isOk shouldBe false

      algorithm.verify(sign, randomData).isOk shouldBe false
    }

    "correctly work with signer and checker" in {
      val algo = EcdsaJS.signAlgo
      val keys = algo.generateKeyPair().extract
      val signer = algo.signer(keys)

      val data = rndByteVector(10)
      val sign = signer.sign(data).extract

      algo.checker.check(sign, data).isOk shouldBe true

      val randomSign = signer.sign(rndByteVector(10)).extract
      algo.checker.check(randomSign, data).isOk shouldBe false
    }

    "throw an errors on invalid data" in {
      val algo = EcdsaJS.signAlgo
      val keys = algo.generateKeyPair().extract
      val signer = algo.signer(keys)
      val data = rndByteVector(10)

      val sign = signer.sign(data).extract

      the[CryptoErr] thrownBy algo.checker.check(sign.copy(sign = rndByteVector(10)), data).value.flatMap(_.toTry).get
      the[CryptoErr] thrownBy algo.checker.check(sign.copy(publicKey = sign.publicKey.copy(value = rndByteVector(10))), data).value.flatMap(_.toTry).get
    }
  }
}
