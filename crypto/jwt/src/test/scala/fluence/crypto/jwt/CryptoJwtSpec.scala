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

package fluence.crypto.jwt

import cats.Id
import fluence.codec.PureCodec
import fluence.crypto.{DumbCrypto, KeyPair}
import io.circe.{Json, JsonNumber, JsonObject}
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

class CryptoJwtSpec extends WordSpec with Matchers {
  "CryptoJwt" should {
    val keys: Seq[KeyPair] =
      Stream.from(0).map(ByteVector.fromInt(_)).map(i ⇒ KeyPair.fromByteVectors(i, i))

    val cryptoJwt =
      new CryptoJwt[JsonNumber, JsonObject](
        PureCodec.liftFunc(n ⇒ KeyPair.Public(ByteVector.fromInt(n._1.toInt.get)))
      )

    val algo = DumbCrypto.signAlgo
    import algo.checker

    "be a total bijection for valid JWT" in {
      val kp = keys.head
      val str = cryptoJwt
        .writer(algo.signer(kp))
        .unsafe((JsonNumber.fromIntegralStringUnsafe("0"), JsonObject("test" → Json.fromString("value of test"))))

      str.count(_ == '.') shouldBe 2

      cryptoJwt.reader(checker).unsafe(str)._2.kleisli.run("test").get shouldBe Json.fromString("value of test")
    }

    "fail with wrong signer" in {
      val kp = keys.tail.head
      val str = cryptoJwt
        .writer(algo.signer(kp))
        .runEither[Id](
          (JsonNumber.fromIntegralStringUnsafe("0"), JsonObject("test" → Json.fromString("value of test")))
        )

      str.isLeft shouldBe true
    }

    "fail with wrong signature" in {
      val kp = keys.head
      val str = cryptoJwt
        .writer(algo.signer(kp))
        .unsafe((JsonNumber.fromIntegralStringUnsafe("0"), JsonObject("test" → Json.fromString("value of test"))))

      cryptoJwt.reader(checker).runEither[Id](str + "m").isLeft shouldBe true
    }
  }
}
