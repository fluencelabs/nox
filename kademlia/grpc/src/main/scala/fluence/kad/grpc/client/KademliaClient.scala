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

import java.net.InetAddress
import java.time.Instant

import cats.syntax.functor._
import cats.{ Monad, ~> }
import com.google.protobuf.ByteString
import fluence.kad.grpc
import fluence.kad.protocol.{ Contact, KademliaRpc, Key, Node }
import io.grpc.{ CallOptions, ManagedChannel }

import scala.concurrent.Future
import scala.language.{ higherKinds, implicitConversions }

/**
 * Implementation of KademliaClient over GRPC, with Task and Contact.
 *
 * @param stub GRPC Kademlia Stub
 */
class KademliaClient[F[_] : Monad](stub: grpc.KademliaGrpc.KademliaStub, run: Future ~> F) extends KademliaRpc[F, Contact] {

  private implicit def nToNc(n: grpc.Node): Node[Contact] = Node[Contact](
    Key(n.id.toByteArray),
    Instant.now(),
    Contact(
      InetAddress.getByAddress(n.ip.toByteArray),
      n.port
    )
  )

  /**
   * Ping the contact, get its actual Node status, or fail
   *
   * @return
   */
  override def ping(): F[Node[Contact]] =
    for {
      n ← run(stub.ping(grpc.PingRequest()))
    } yield n: Node[Contact]

  /**
   * Perform a local lookup for a key, return K closest known nodes
   *
   * @param key Key to lookup
   * @return
   */
  override def lookup(key: Key, numberOfNodes: Int): F[Seq[Node[Contact]]] =
    for {
      res ← run(stub.lookup(grpc.LookupRequest(ByteString.copyFrom(key.id), numberOfNodes)))
    } yield res.nodes.map(n ⇒ n: Node[Contact])

  /**
   * Perform an iterative lookup for a key, return K closest known nodes
   *
   * @param key Key to lookup
   * @return
   */
  override def lookupIterative(key: Key, numberOfNodes: Int): F[Seq[Node[Contact]]] =
    for {
      res ← run(stub.lookupIterative(grpc.LookupRequest(ByteString.copyFrom(key.id), numberOfNodes)))
    } yield res.nodes.map(n ⇒ n: Node[Contact])

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): F[Seq[Node[Contact]]] =
    for {
      res ← run(
        stub.lookupAway(grpc.LookupAwayRequest(ByteString.copyFrom(key.id), ByteString.copyFrom(moveAwayFrom.id), numberOfNodes))
      )
    } yield res.nodes.map(n ⇒ n: Node[Contact])
}

object KademliaClient {

  /**
   * Shorthand to register KademliaClient inside NetworkClient.
   *
   * @param channel Channel to remote node
   * @param callOptions Call options
   * @return
   */
  def register[F[_] : Monad]()(channel: ManagedChannel, callOptions: CallOptions)(implicit run: Future ~> F): KademliaClient[F] =
    new KademliaClient(new grpc.KademliaGrpc.KademliaStub(channel, callOptions), run)

}
