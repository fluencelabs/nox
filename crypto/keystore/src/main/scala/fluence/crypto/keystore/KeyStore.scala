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

package fluence.crypto.keystore

import cats.syntax.compose._
import fluence.codec.PureCodec
import fluence.codec.bits.BitsCodecs
import fluence.codec.circe.CirceCodecs
import fluence.crypto.KeyPair
import io.circe.{HCursor, Json}
import scodec.bits.{Bases, ByteVector}

import scala.language.higherKinds

/**
 * Json example:
 * {
 *   "keystore" : {
 *     "secret" : "SFcDtZClfcxx75w9xJpQgBm09d6h9tVmVUEgHYxlews=",
 *     "public" : "AlTBivFrIYe++9Me4gr4R11BtRzjZ2WXZGDNWD/bEPka"
 *   }
 * }
 */
object KeyStore {

  private val alphabet = Bases.Alphabets.Base64Url

  private object Field {
    val Keystore = "keystore"
    val Secret = "secret"
    val Public = "public"
  }

  // Codec for a tuple of already serialized public and secret keys to json
  private val pubSecJsonCodec: PureCodec[(String, String), Json] =
    CirceCodecs.circeJsonCodec(
      {
        case (pub, sec) ⇒
          Json.obj(
            (
              Field.Keystore,
              Json.obj(
                (Field.Secret, Json.fromString(sec)),
                (Field.Public, Json.fromString(pub))
              )
            )
          )
      },
      (c: HCursor) ⇒
        for {
          sec ← c.downField(Field.Keystore).downField(Field.Secret).as[String]
          pub ← c.downField(Field.Keystore).downField(Field.Public).as[String]
        } yield (pub, sec)
    )

  // ByteVector to/from String, with the chosen alphabet
  private val vecToStr = BitsCodecs.base64AlphabetToVector(alphabet).swap

  implicit val keyPairJsonCodec: PureCodec[KeyPair, Json] =
    PureCodec.liftB[KeyPair, (ByteVector, ByteVector)](
      kp ⇒ (kp.publicKey.value, kp.secretKey.value), {
        case (pub, sec) ⇒ KeyPair(KeyPair.Public(pub), KeyPair.Secret(sec))
      }
    ) andThen (vecToStr split vecToStr) andThen pubSecJsonCodec

  implicit val keyStoreJsonStringCodec: PureCodec[KeyPair, String] =
    keyPairJsonCodec andThen CirceCodecs.circeJsonParseCodec

}
