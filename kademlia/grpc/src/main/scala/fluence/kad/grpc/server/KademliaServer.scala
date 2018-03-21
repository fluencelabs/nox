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

import cats.effect.IO
import cats.instances.stream._
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.kad.grpc._
import fluence.kad.protocol
import fluence.kad.protocol.{Contact, KademliaRpc, Key}
import fluence.codec.pb.ProtobufCodecs._

import scala.concurrent.Future
import scala.language.implicitConversions

// TODO: cover with tests
class KademliaServer(kademlia: KademliaRpc[Contact])(
  implicit
  codec: Codec[IO, protocol.Node[Contact], Node]
) extends KademliaGrpc.Kademlia {

  private val streamCodec = Codec.codec[IO, Stream[protocol.Node[Contact]], Stream[Node]]

  private val keyCodec = Codec.codec[IO, Key, ByteString]

  override def ping(request: PingRequest): Future[Node] =
    kademlia.ping().flatMap(codec.encode).unsafeToFuture()

  override def lookup(request: LookupRequest): Future[NodesResponse] =
    (
      for {
        key ← keyCodec.decode(request.key)
        ns ← kademlia
          .lookup(key, request.numberOfNodes)
        resp ← streamCodec.encode(ns.toStream)
      } yield NodesResponse(resp)
    ).unsafeToFuture()

  override def lookupAway(request: LookupAwayRequest): Future[NodesResponse] =
    (
      for {
        key ← keyCodec.decode(request.key)
        moveAwayKey ← keyCodec.decode(request.moveAwayFrom)
        ns ← kademlia
          .lookupAway(key, moveAwayKey, request.numberOfNodes)
        resp ← streamCodec.encode(ns.toStream)
      } yield NodesResponse(resp)
    ).unsafeToFuture()

}
