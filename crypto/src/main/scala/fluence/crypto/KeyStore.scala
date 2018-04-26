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

import cats.Monad
import cats.data.EitherT
import io.circe.parser.decode
import io.circe.{Decoder, Encoder, HCursor, Json}
import scodec.bits.{Bases, ByteVector}

import scala.language.higherKinds

case class KeyStore(keyPair: KeyPair)

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

  implicit val encodeKeyStorage: Encoder[KeyStore] = (ks: KeyStore) ⇒
    Json.obj(
      (
        Field.Keystore,
        Json.obj(
          (Field.Secret, Json.fromString(ks.keyPair.secretKey.value.toBase64(alphabet))),
          (Field.Public, Json.fromString(ks.keyPair.publicKey.value.toBase64(alphabet)))
        )
      )
  )

  implicit val decodeKeyStorage: Decoder[Option[KeyStore]] =
    (c: HCursor) ⇒
      for {
        secret ← c.downField(Field.Keystore).downField(Field.Secret).as[String]
        public ← c.downField(Field.Keystore).downField(Field.Public).as[String]
      } yield {
        for {
          secret ← ByteVector.fromBase64(secret, alphabet)
          public ← ByteVector.fromBase64(public, alphabet)
        } yield KeyStore(KeyPair.fromByteVectors(public, secret))
    }

  def fromBase64[F[_]: Monad](base64: String): EitherT[F, IllegalArgumentException, KeyStore] =
    for {
      jsonStr ← ByteVector.fromBase64(base64, alphabet) match {
        case Some(bv) ⇒ EitherT.pure[F, IllegalArgumentException](new String(bv.toArray))
        case None ⇒
          EitherT.leftT(
            new IllegalArgumentException("'" + base64 + "' is not a valid base64.")
          )
      }
      keyStore ← decode[Option[KeyStore]](jsonStr) match {
        case Right(Some(ks)) ⇒ EitherT.pure[F, IllegalArgumentException](ks)
        case Right(None) ⇒
          EitherT.leftT[F, KeyStore](new IllegalArgumentException("'" + base64 + "' is not a valid key store."))
        case Left(err) ⇒
          EitherT.leftT[F, KeyStore](new IllegalArgumentException("'" + base64 + "' is not a valid key store.", err))
      }
    } yield keyStore
}
