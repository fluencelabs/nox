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

import cats.{Id, Monad}
import cats.data.EitherT
import fluence.codec.PureCodec
import fluence.crypto.signature.{SignAlgo, Signature, SignatureChecker, Signer}
import fluence.crypto.{Crypto, CryptoError, KeyPair}
import io.circe.{Json, JsonNumber, JsonObject}
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.language.higherKinds

class CryptoJwtSpec extends WordSpec with Matchers {
  "CryptoJwt" should {
    val keys: Seq[KeyPair] =
      Stream.from(0).map(ByteVector.fromInt(_)).map(i ⇒ KeyPair.fromByteVectors(i, i))

    val cryptoJwt =
      new CryptoJwt[JsonNumber, JsonObject](
        PureCodec.liftFunc(n ⇒ KeyPair.Public(ByteVector.fromInt(n._1.toInt.get)))
      )

    implicit val checker: SignAlgo.CheckerFn =
      publicKey ⇒
        new SignatureChecker {
          override def check[F[_]: Monad](signature: Signature, plain: ByteVector): EitherT[F, CryptoError, Unit] =
            EitherT.cond[F](signature.sign == plain.reverse, (), CryptoError("Signatures mismatch"))
      }

    val signer: SignAlgo.SignerFn =
      keyPair ⇒ Signer(keyPair.publicKey, Crypto.liftFunc(plain ⇒ Signature(plain.reverse)))

    "be a total bijection for valid JWT" in {
      val kp = keys.head
      val str = cryptoJwt
        .writer(signer(kp))
        .unsafe((JsonNumber.fromIntegralStringUnsafe("0"), JsonObject("test" → Json.fromString("value. of test"))))

      str.count(_ == '.') shouldBe 2

      cryptoJwt.reader(checker).unsafe(str)._2.kleisli.run("test").get shouldBe Json.fromString("value. of test")
    }

    "fail with wrong signer" in {
      val kp = keys.tail.head
      val str = cryptoJwt
        .writer(signer(kp))
        .runEither[Id](
          (JsonNumber.fromIntegralStringUnsafe("0"), JsonObject("test" → Json.fromString("value of test")))
        )

      str.isLeft shouldBe true
    }

    "fail with wrong signature" in {
      val kp = keys.head
      val str = cryptoJwt
        .writer(signer(kp))
        .unsafe((JsonNumber.fromIntegralStringUnsafe("0"), JsonObject("test" → Json.fromString("value of test"))))

      cryptoJwt.reader(checker).runEither[Id](str + "m").isLeft shouldBe true
    }
  }
}
