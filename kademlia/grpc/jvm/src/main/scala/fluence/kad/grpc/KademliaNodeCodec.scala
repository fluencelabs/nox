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

import java.time.Instant

import cats.data.EitherT
import cats.Monad
import cats.syntax.compose._
import com.google.protobuf.ByteString
import fluence.codec.{CodecError, PureCodec}
import fluence.crypto.SignAlgo.CheckerFn
import fluence.kad.protocol
import fluence.kad.protocol.{Contact, Key}
import fluence.codec.pb.ProtobufCodecs._
import fluence.protobuf.kad.Node
import fluence.transport.grpc.KeyProtobufCodecs._

import scala.language.higherKinds

object KademliaNodeCodec {
  implicit def pureCodec(implicit checkerFn: CheckerFn): PureCodec[fluence.kad.protocol.Node[Contact], Node] = {
    val keyCodec = PureCodec.codec[Key, ByteString]
    val contactCodec = PureCodec.codec[Contact, Array[Byte]] andThen PureCodec.codec[Array[Byte], ByteString]

    PureCodec.build(
      new PureCodec.Func[fluence.kad.protocol.Node[Contact], Node] {
        override def apply[F[_]: Monad](input: protocol.Node[Contact]): EitherT[F, CodecError, Node] =
          for {
            id ← keyCodec.direct[F](input.key)
            contact ← contactCodec.direct[F](input.contact)
          } yield Node(id, contact)
      },
      new PureCodec.Func[Node, fluence.kad.protocol.Node[Contact]] {
        override def apply[F[_]: Monad](input: Node): EitherT[F, CodecError, protocol.Node[Contact]] =
          for {
            id ← keyCodec.inverse[F](input.id)
            contact ← contactCodec.inverse[F](input.contact)
            _ ← Key
              .checkPublicKey[F](id, contact.publicKey)
              .leftMap(t ⇒ CodecError(s"Decoding node with key=$id failed", causedBy = Some(t)))
          } yield
            protocol.Node[Contact](
              id,
              Instant
                .now(), // TODO: remove Instant.now, it is incorrect and should never be set in codec: decoding a Node doesn't mean it's being freshly seen
              contact
            )
      }
    )
  }
}
