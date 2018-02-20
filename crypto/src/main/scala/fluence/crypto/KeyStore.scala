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

import fluence.crypto.keypair.KeyPair
import io.circe.{ Decoder, Encoder, HCursor, Json }
import scodec.bits.{ Bases, ByteVector }
import io.circe.parser.decode

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
  implicit val encodeKeyStorage: Encoder[KeyStore] = new Encoder[KeyStore] {
    final def apply(ks: KeyStore): Json = Json.obj(("keystore", Json.obj(
      ("secret", Json.fromString(ks.keyPair.secretKey.value.toBase64(alphabet))),
      ("public", Json.fromString(ks.keyPair.publicKey.value.toBase64(alphabet))))))
  }

  implicit val decodeKeyStorage: Decoder[Option[KeyStore]] = new Decoder[Option[KeyStore]] {
    final def apply(c: HCursor): Decoder.Result[Option[KeyStore]] =
      for {
        secret ← c.downField("keystore").downField("secret").as[String]
        public ← c.downField("keystore").downField("public").as[String]
      } yield {
        for {
          secret ← ByteVector.fromBase64(secret, alphabet)
          public ← ByteVector.fromBase64(public, alphabet)
        } yield KeyStore(KeyPair.fromByteVectors(public, secret))
      }
  }

  def fromBase64(base64: String): KeyStore = {
    val jsonStr = ByteVector.fromBase64(base64, alphabet) match {
      case Some(bv) ⇒ new String(bv.toArray)
      case None ⇒
        throw new IllegalArgumentException("'" + base64 + "' is not a valid base64.")
    }
    decode[Option[KeyStore]](jsonStr) match {
      case Right(Some(ks)) ⇒ ks
      case Right(None)     ⇒ throw new IllegalArgumentException("'" + base64 + "' is not a valid key store.")
      case Left(err)       ⇒ throw new IllegalArgumentException("'" + base64 + "' is not a valid key store.", err)
    }
  }
}
