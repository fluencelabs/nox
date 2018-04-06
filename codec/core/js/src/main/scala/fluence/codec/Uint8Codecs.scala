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

import scala.scalajs.js.JSConverters._
import scala.scalajs.js.typedarray.Uint8Array

object Uint8Codecs {

  implicit val byteArrayUint8Array: PureCodec[Uint8Array, Array[Byte]] =
    PureCodec.liftB(
      uint8 ⇒ uint8.toArray.map(_.toByte),
      arr ⇒ new Uint8Array(arr.toJSArray)
    )
}
