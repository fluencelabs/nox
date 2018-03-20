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

package fluence.codec.pb

import cats.{Applicative, MonadError}
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.kad.protocol.Key
import scodec.bits.ByteVector

import scala.language.higherKinds

object ProtobufCodecs {

  implicit def byteVectorByteString[F[_]: Applicative]: Codec[F, ByteString, ByteVector] =
    Codec.pure(
      str ⇒ ByteVector(str.toByteArray),
      vec ⇒ ByteString.copyFrom(vec.toArray)
    )

  // TODO: more precise error
  implicit def keyByteString[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, ByteString, Key] =
    byteVectorByteString[F] andThen Key.vectorCodec.swap
}
