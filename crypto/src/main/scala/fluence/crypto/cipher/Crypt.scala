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

package fluence.crypto.cipher

import cats.Monad
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.codec.Codec

import scala.language.higherKinds

/**
 * Base interface for encrypting/decrypting.
 * TODO: switch to Codec; notice that Crypt provides effect: decrypt may fail
 *
 * @tparam P The type of plain text, input
 * @tparam C The type of cipher text, output
 */
trait Crypt[F[_], P, C] {

  def encrypt(plainText: P): F[C]

  def decrypt(cipherText: C): F[P]

}

object Crypt {

  def apply[F[_], O, B](implicit crypt: Crypt[F, O, B]): Crypt[F, O, B] = crypt

  implicit def transform[F[_] : Monad, K, K1, V, V1](
      crypt: Crypt[F, K, V]
  )(
      implicit
      plainTextCodec: Codec[F, K1, K],
      cipherTextCodec: Codec[F, V1, V]
  ): Crypt[F, K1, V1] =
    new Crypt[F, K1, V1] {

      override def encrypt(plainText: K1): F[V1] =
        for {
          pt ← plainTextCodec.encode(plainText)
          v ← crypt.encrypt(pt)
          v1 ← cipherTextCodec.decode(v)
        } yield v1

      override def decrypt(cipherText: V1): F[K1] =
        for {
          ct ← cipherTextCodec.encode(cipherText)
          v ← crypt.decrypt(ct)
          v1 ← plainTextCodec.decode(v)
        } yield v1

    }

}
