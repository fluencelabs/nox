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
import cats.{ MonadError, ~> }
import com.google.protobuf.ByteString
import fluence.kad.grpc._
import fluence.kad.protocol
import fluence.kad.protocol.{ Contact, KademliaRpc, Key }

import scala.concurrent.Future
import scala.language.{ higherKinds, implicitConversions }

// TODO: cover with tests
class KademliaServer[F[_]](kademlia: KademliaRpc[F, Contact])(implicit F: MonadError[F, Throwable], run: F ~> Future) extends KademliaGrpc.Kademlia {

  private implicit def ncToNode(nc: protocol.Node[Contact]): Node =
    Node(id = ByteString.copyFrom(nc.key.id), ByteString.copyFrom(nc.contact.ip.getAddress), nc.contact.port)

  override def ping(request: PingRequest): Future[Node] =
    run(
      kademlia.ping().map(nc ⇒ nc: Node)
    )

  override def lookup(request: LookupRequest): Future[NodesResponse] =
    run(
      kademlia
        .lookup(Key(request.key.toByteArray), request.numberOfNodes)
        .map(_.map(nc ⇒ nc: Node))
        .map(NodesResponse(_))
    )

  override def lookupAway(request: LookupAwayRequest): Future[NodesResponse] =
    run(
      kademlia
        .lookupAway(Key(request.key.toByteArray), Key(request.moveAwayFrom.toByteArray), request.numberOfNodes)
        .map(_.map(nc ⇒ nc: Node))
        .map(NodesResponse(_))
    )

  override def lookupIterative(request: LookupRequest): Future[NodesResponse] =
    run(
      kademlia
        .lookupIterative(Key(request.key.toByteArray), request.numberOfNodes)
        .map(_.map(nc ⇒ nc: Node))
        .map(NodesResponse(_))
    )

}
