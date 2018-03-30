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
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.codec.Codec
import fluence.crypto.SignAlgo.CheckerFn
import fluence.kad.grpc.facade.Node
import fluence.kad.protocol
import fluence.kad.protocol.{Contact, Key}

import scala.scalajs.js.typedarray.Uint8Array
import scala.scalajs.js.JSConverters._
import scala.language.higherKinds

//TODO merge it with KademliaNodeCodec in jvm project
object KademliaNodeCodec {
  implicit def codec[F[_]](
    implicit F: MonadError[F, Throwable],
    checkerFn: CheckerFn
  ): Codec[F, fluence.kad.protocol.Node[Contact], Node] = {

    def contactFromBase64(str: String) =
      Contact
        .readB64seed[F](str)
        .value
        .flatMap(F.fromEither) // TODO err: crypto

    def keyFromBytes(arr: Array[Byte]) = Key.fromBytes.runF[F](arr)

    def encode: fluence.kad.protocol.Node[Contact] ⇒ F[Node] =
      obj ⇒
        Node(
          id = new Uint8Array(obj.key.id.toJSArray),
          contact = new Uint8Array(obj.contact.b64seed.getBytes().toJSArray)
        ).pure[F]

    def decode: Node ⇒ F[fluence.kad.protocol.Node[Contact]] =
      binary ⇒ {
        for {
          id ← JSCodecs.byteVectorUint8Array.encode(binary.id)
          k ← keyFromBytes(id.toArray)
          contact ← JSCodecs.byteVectorUint8Array.encode(binary.contact)
          c ← contactFromBase64(new String(contact.toArray))
          _ ← if (Key.checkPublicKey(k, c.publicKey)) F.pure(())
          else
            F.raiseError(new IllegalArgumentException("Key doesn't conform to signature")) // TODO err: crypto -- keys mismatch
        } yield
          protocol.Node[Contact](
            k,
            // TODO: consider removing Instant.now(). It could be really incorrect, as nodes taken from lookup replies are not seen at the moment
            Instant.now(),
            c
          )
      }

    Codec(
      encode,
      decode
    )
  }
}
