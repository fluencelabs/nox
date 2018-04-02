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

package fluence.crypto.signature

import scodec.bits.ByteVector

/** Container for a signature. */
case class Signature private (sign: ByteVector) extends AnyVal {
  def bytes: Array[Byte] = sign.toArray
}

object Signature {

  def apply(sign: ByteVector): Signature = new Signature(sign)

  def apply(sign: Array[Byte]): Signature = new Signature(ByteVector(sign))

}
