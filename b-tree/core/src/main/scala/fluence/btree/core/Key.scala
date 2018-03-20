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

package fluence.btree.core

import cats.Applicative
import fluence.codec.Codec
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Ciphered btree key
 */
case class Key(bytes: Array[Byte]) extends AnyVal {

  def copy: Key = Key(bytes.clone())

  override def toString: String =
    if (bytes.isEmpty) "Key(empty)" else s"Key(${bytes.length} bytes, 0x${ByteVector.view(bytes).toHex})"

}

object Key {

  implicit def keyCodec[F[_]: Applicative]: Codec[F, Key, Array[Byte]] = Codec.pure(_.bytes, b ⇒ Key(b))

  implicit class KeyOps(originKey: Key) {

    def isEmpty: Boolean = originKey.bytes.isEmpty

    def toByteVector: ByteVector = ByteVector(originKey.bytes)

  }
}
