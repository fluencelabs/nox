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

package fluence.crypto.keypair

import scodec.bits.ByteVector

case class KeyPair(publicKey: KeyPair.Public, secretKey: KeyPair.Secret)

object KeyPair {

  case class Public(value: ByteVector) extends AnyVal {
    def bytes: Array[Byte] = value.toArray
  }

  case class Secret(value: ByteVector) extends AnyVal {
    def bytes: Array[Byte] = value.toArray
  }

  def fromBytes(pk: Array[Byte], sk: Array[Byte]): KeyPair = fromByteVectors(ByteVector(pk), ByteVector(sk))
  def fromByteVectors(pk: ByteVector, sk: ByteVector): KeyPair = KeyPair(Public(pk), Secret(sk))
}
