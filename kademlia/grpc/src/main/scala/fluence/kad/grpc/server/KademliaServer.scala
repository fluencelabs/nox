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

package fluence.kad.grpc.server

import cats.syntax.functor._
import cats.instances.stream._
import cats.{~>, MonadError}
import cats.syntax.flatMap._
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.kad.grpc._
import fluence.kad.protocol
import fluence.kad.protocol.{Contact, KademliaRpc, Key}
import fluence.codec.pb.ProtobufCodecs._

import scala.concurrent.Future
import scala.language.{higherKinds, implicitConversions}

// TODO: cover with tests
class KademliaServer[F[_]](kademlia: KademliaRpc[F, Contact])(
  implicit
  F: MonadError[F, Throwable],
  codec: Codec[F, protocol.Node[Contact], Node],
  run: F ~> Future
) extends KademliaGrpc.Kademlia {

  private val streamCodec = Codec.codec[F, Stream[protocol.Node[Contact]], Stream[Node]]

  private val keyCodec = Codec.codec[F, Key, ByteString]

  override def ping(request: PingRequest): Future[Node] =
    run(
      kademlia.ping().flatMap(codec.encode)
    )

  override def lookup(request: LookupRequest): Future[NodesResponse] =
    run(
      for {
        key ← keyCodec.decode(request.key)
        ns ← kademlia
          .lookup(key, request.numberOfNodes)
        resp ← streamCodec.encode(ns.toStream)
      } yield NodesResponse(resp)
    )

  override def lookupAway(request: LookupAwayRequest): Future[NodesResponse] =
    run(
      for {
        key ← keyCodec.decode(request.key)
        moveAwayKey ← keyCodec.decode(request.moveAwayFrom)
        ns ← kademlia
          .lookupAway(key, moveAwayKey, request.numberOfNodes)
        resp ← streamCodec.encode(ns.toStream)
      } yield NodesResponse(resp)
    )

}
