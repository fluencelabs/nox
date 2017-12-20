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

import java.nio.ByteBuffer

/**
 * No operation implementation. Just convert the element to bytes back and forth without any cryptography.
 */
class NoOpCrypt[T](serializer: T ⇒ Array[Byte], deserializer: Array[Byte] ⇒ T) extends Crypt[T, Array[Byte]] {

  def encrypt(plainText: T): Array[Byte] = serializer(plainText)

  def decrypt(cipherText: Array[Byte]): T = deserializer(cipherText)

}

object NoOpCrypt {

  val forString: NoOpCrypt[String] = apply[String](
    serializer = _.getBytes,
    deserializer = bytes ⇒ new String(bytes))

  val forLong: NoOpCrypt[Long] = apply[Long](
    serializer = ByteBuffer.allocate(java.lang.Long.BYTES).putLong(_).array(),
    deserializer = bytes ⇒ ByteBuffer.wrap(bytes).getLong()
  )

  def apply[T](serializer: T ⇒ Array[Byte], deserializer: Array[Byte] ⇒ T): NoOpCrypt[T] =
    new NoOpCrypt(serializer, deserializer)

}
