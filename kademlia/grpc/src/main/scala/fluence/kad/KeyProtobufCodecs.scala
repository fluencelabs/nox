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

package fluence.kad

import cats.syntax.compose._
import com.google.protobuf.ByteString
import fluence.codec.PureCodec
import fluence.kad.protocol.Key
import scodec.bits.ByteVector
import fluence.codec.pb.ProtobufCodecs._
import fluence.codec.bits.BitsCodecs._

object KeyProtobufCodecs {
  implicit val protobufKeyCodec: PureCodec[Key, ByteString] =
    PureCodec[Key, ByteVector] andThen PureCodec[ByteVector, Array[Byte]] andThen PureCodec[Array[Byte], ByteString]
}
