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

import java.nio.ByteBuffer

import cats.Applicative
import cats.syntax.applicative._

import scala.language.higherKinds

/**
 * No operation implementation. Just convert the element to bytes back and forth without any cryptography.
 */
class NoOpCrypt[F[_], T](serializer: T ⇒ F[Array[Byte]], deserializer: Array[Byte] ⇒ F[T])
    extends Crypt[F, T, Array[Byte]] {

  def encrypt(plainText: T): F[Array[Byte]] = serializer(plainText)

  def decrypt(cipherText: Array[Byte]): F[T] = deserializer(cipherText)

}

object NoOpCrypt {

  def forString[F[_]: Applicative]: NoOpCrypt[F, String] =
    apply[F, String](serializer = _.getBytes.pure[F], deserializer = bytes ⇒ new String(bytes).pure[F])

  def forLong[F[_]: Applicative]: NoOpCrypt[F, Long] =
    apply[F, Long](
      serializer = ByteBuffer.allocate(java.lang.Long.BYTES).putLong(_).array().pure[F],
      deserializer = bytes ⇒ ByteBuffer.wrap(bytes).getLong().pure[F]
    )

  def apply[F[_], T](serializer: T ⇒ F[Array[Byte]], deserializer: Array[Byte] ⇒ F[T]): NoOpCrypt[F, T] =
    new NoOpCrypt(serializer, deserializer)

}
