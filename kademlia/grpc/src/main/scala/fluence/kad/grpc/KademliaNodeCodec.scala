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

import cats.MonadError
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.crypto.signature.SignatureChecker
import fluence.kad.protocol
import fluence.kad.protocol.{ Contact, Key }

import scala.language.higherKinds

object KademliaNodeCodec {
  implicit def apply[F[_]](implicit F: MonadError[F, Throwable], checker: SignatureChecker): Codec[F, fluence.kad.protocol.Node[Contact], Node] =
    Codec(
      obj ⇒
        obj.contact.b64seed[F].value.flatMap(F.fromEither).map(contact ⇒
          Node(
            id = ByteString.copyFrom(obj.key.id),
            contact = ByteString.copyFrom(contact.getBytes)
          )),
      binary ⇒
        for {
          k ← Key.fromBytes[F](binary.id.toByteArray)
          c ← Contact.readB64seed[F](new String(binary.contact.toByteArray), checker).value.flatMap(F.fromEither)
        } yield protocol.Node[Contact](
          k,
          // TODO: consider removing Instant.now(). It could be really incorrect, as nodes taken from lookup replies are not seen at the moment
          Instant.now(),
          c
        ))
}
