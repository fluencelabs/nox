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

package fluence.kad.grpc.client

import cats.Monad
import cats.data.EitherT
import fluence.codec.{CodecError, PureCodec}
import scalapb.{GeneratedMessage, GeneratedMessageCompanion, Message}

import scala.language.higherKinds

object ProtobufCodec {

  def protobufDynamicCodec[A <: GeneratedMessage with Message[A]](
    gen: GeneratedMessageCompanion[A]
  ): fluence.codec.PureCodec.Func[Array[Byte], A] = {
    new PureCodec.Func[Array[Byte], A] {
      override def apply[F[_]](
        input: Array[Byte]
      )(implicit F: Monad[F]): EitherT[F, CodecError, A] = {
        EitherT.rightT(input).map(gen.parseFrom)
      }
    }
  }

  val generatedMessageCodec: fluence.codec.PureCodec.Func[GeneratedMessage, Array[Byte]] =
    new PureCodec.Func[GeneratedMessage, Array[Byte]] {
      override def apply[F[_]](input: GeneratedMessage)(implicit F: Monad[F]): EitherT[F, CodecError, Array[Byte]] = {
        EitherT.rightT(input).map(_.toByteString).map(_.toByteArray)
      }
    }
}
