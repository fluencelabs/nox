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

import java.net.InetAddress
import java.time.Instant

import cats.ApplicativeError
import cats.syntax.functor._
import cats.syntax.applicative._
import fluence.codec.Codec
import fluence.kad.protocol
import fluence.kad.protocol.{ Contact, Key }
import com.google.protobuf.ByteString

import scala.language.higherKinds

object KademliaNodeCodec {
  implicit def apply[F[_]](implicit F: ApplicativeError[F, Throwable]): Codec[F, fluence.kad.protocol.Node[Contact], Node] =
    Codec(
      obj ⇒ Node(
        id = ByteString.copyFrom(obj.key.id),
        ByteString.copyFrom(obj.contact.ip.getAddress),
        obj.contact.port
      ).pure[F],
      binary ⇒ Key.fromBytes(binary.id.toByteArray)
        .map(k ⇒ protocol.Node[Contact](
          k,
          // TODO: consider removing Instant.now(). It could be really incorrect, as nodes taken from lookup replies are not seen at the moment
          Instant.now(),
          Contact(
            InetAddress.getByAddress(binary.ip.toByteArray),
            binary.port
          )
        ))
    )
}
