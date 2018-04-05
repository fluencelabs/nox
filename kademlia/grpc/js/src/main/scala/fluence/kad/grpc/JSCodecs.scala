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

package fluence.kad.grpc

import cats.syntax.compose._
import fluence.codec.PureCodec
import fluence.kad.protocol.Key
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.scalajs.js.JSConverters._
import scala.scalajs.js.typedarray.Uint8Array

object JSCodecs {
  implicit def byteVectorUint8Array: PureCodec[Uint8Array, ByteVector] =
    PureCodec(
      jsUnsignedArray ⇒ {
        ByteVector(jsUnsignedArray.toArray.map(_.toByte))
      },
      vec ⇒ new Uint8Array(vec.toArray.map(_.toShort).toJSArray)
    )

  // TODO: more precise error
  implicit def keyUint8Array: PureCodec[Uint8Array, Key] =
    byteVectorUint8Array andThen Key.keyVectorCodec.swap
}
