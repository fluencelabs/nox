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

import fluence.codec.PureCodec.{lift, liftEither}
import scodec.bits.{Bases, ByteVector}
import scala.language.higherKinds

private[codec] trait PureCodecInstances extends BifuncEInstances {

  implicit val byteArrayToVector: PureCodec[Array[Byte], ByteVector] =
    lift(ByteVector.apply, _.toArray)

  implicit val base64ToVector: PureCodec[String, ByteVector] =
    liftEither(
      str ⇒
        ByteVector
          .fromBase64(str, Bases.Alphabets.Base64Url)
          .toRight(CodecError(s"Given string is not valid b64: $str")),
      vec ⇒ Right(vec.toBase64(Bases.Alphabets.Base64Url))
    )

}
