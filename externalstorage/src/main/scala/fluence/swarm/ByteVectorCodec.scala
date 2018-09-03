/*
 * Copyright (C) 2018  Fluence Labs Limited
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

package fluence.swarm
import io.circe._
import scodec.bits.ByteVector

object ByteVectorCodec {

  /**
   * Every byte array in JSON is `0x` prefixed in Swarm.
   */
  implicit final val encodeByteVector: Encoder[ByteVector] = (bv: ByteVector) => Json.fromString("0x" + bv.toHex)

  implicit final val decodeByteVector: Decoder[ByteVector] = {
    Decoder.decodeString.emap { str =>
      ByteVector.fromHexDescriptive(str).left.map(t => "ByteVector")
    }
  }
}
