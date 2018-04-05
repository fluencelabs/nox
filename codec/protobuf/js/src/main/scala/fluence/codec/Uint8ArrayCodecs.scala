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

package fluence.codec

import scodec.bits.ByteVector

import scala.language.higherKinds

import scala.scalajs.js.typedarray.Uint8Array
import scala.scalajs.js.JSConverters._

object Uint8ArrayCodecs {

  implicit val byteVectorUint8Array: PureCodec[Uint8Array, ByteVector] =
    PureCodec.liftB(
      uint8 ⇒ ByteVector(uint8.toArray.map(_.toByte)),
      vec ⇒ new Uint8Array(vec.toArray.map(_.toShort).toJSArray)
    )

  implicit val byteArrayUint8Array: PureCodec[Uint8Array, Array[Byte]] =
    PureCodec.liftB(
      uint8 ⇒ uint8.toArray.map(_.toByte),
      arr ⇒ new Uint8Array(arr.toJSArray)
    )
}
