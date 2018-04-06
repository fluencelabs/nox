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

package fluence.transport.grpc

import cats.syntax.compose._
import fluence.codec.PureCodec
import fluence.kad.protocol.Key
import scodec.bits.ByteVector
import fluence.codec.bits.BitsUint8Codecs._

import scala.scalajs.js.typedarray.Uint8Array

object KeyUint8Codecs {
  implicit val uint8KeyCodec: PureCodec[Key, Uint8Array] =
    PureCodec[Key, ByteVector] andThen PureCodec[ByteVector, Uint8Array]
}
