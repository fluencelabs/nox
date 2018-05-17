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

package fluence.transport.websocket

import fluence.codec.PureCodec
import scalapb.{GeneratedMessage, GeneratedMessageCompanion, Message}

import scala.language.higherKinds

object ProtobufCodec {

  /**
   * Codec for converting byte array to protobuf class.
   *
   * @param gen Protobuf class's companion.
   * @return New codec for converting byte array to a specific protobuf class.
   */
  def protobufDynamicCodec[A <: GeneratedMessage with Message[A]](
    gen: GeneratedMessageCompanion[A]
  ): PureCodec.Func[Array[Byte], A] = PureCodec.liftFunc[Array[Byte], A](gen.parseFrom)

  /**
   * Codec for converting protobuf class to a byte array.
   */
  val generatedMessageCodec: PureCodec.Func[GeneratedMessage, Array[Byte]] =
    PureCodec.liftFunc[GeneratedMessage, Array[Byte]](_.toByteString.toByteArray)
}
