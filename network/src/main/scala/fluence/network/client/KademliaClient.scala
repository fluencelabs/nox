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

package fluence.network.client

import java.net.InetAddress
import java.time.Instant

import com.google.protobuf.ByteString
import fluence.kad.{ KademliaRpc, Key, Node }
import fluence.network.{ Contact, proto }
import fluence.network.proto.kademlia._
import io.grpc.{ CallOptions, ManagedChannel }
import monix.eval.Task
import shapeless._

import scala.language.implicitConversions

/**
 * Implementation of KademliaClient over GRPC, with Task and Contact
 * @param stub GRPC Kademlia Stub
 */
class KademliaClient(stub: KademliaGrpc.KademliaStub) extends KademliaRpc[Task, Contact] {

  private implicit def nToNc(n: proto.kademlia.Node): Node[Contact] = Node[Contact](
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
  override def ping(): Task[Node[Contact]] =
    for {
      n ← Task.deferFuture(stub.ping(PingRequest()))
    } yield n: Node[Contact]

  /**
   * Perform a local lookup for a key, return K closest known nodes
   *
   * @param key Key to lookup
   * @return
   */
  override def lookup(key: Key, numberOfNodes: Int): Task[Seq[Node[Contact]]] =
    for {
      res ← Task.deferFuture(stub.lookup(LookupRequest(ByteString.copyFrom(key.id), numberOfNodes)))
    } yield res.nodes.map(n ⇒ n: Node[Contact])

  /**
   * Perform an iterative lookup for a key, return K closest known nodes
   *
   * @param key Key to lookup
   * @return
   */
  override def lookupIterative(key: Key, numberOfNodes: Int): Task[Seq[Node[Contact]]] =
    for {
      res ← Task.deferFuture(stub.lookupIterative(LookupRequest(ByteString.copyFrom(key.id), numberOfNodes)))
    } yield res.nodes.map(n ⇒ n: Node[Contact])

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): Task[Seq[Node[Contact]]] =
    for {
      res ← Task.deferFuture(
        stub.lookupAway(LookupAwayRequest(ByteString.copyFrom(key.id), ByteString.copyFrom(moveAwayFrom.id), numberOfNodes))
      )
    } yield res.nodes.map(n ⇒ n: Node[Contact])
}

object KademliaClient {
  /**
   * Shorthand to register KademliaClient inside NetworkClient
   * @param channel Channel to remote node
   * @param callOptions Call options
   * @return
   */
  def register()(channel: ManagedChannel, callOptions: CallOptions): KademliaClient =
    new KademliaClient(new KademliaGrpc.KademliaStub(channel, callOptions))

  /**
   * Kademlia client summoner for NetworkClient
   * @param client Network client
   * @param sel HList selector
   * @tparam CL HList of network services
   * @return
   */
  def apply[CL <: HList](client: NetworkClient[CL])(implicit sel: ops.hlist.Selector[CL, KademliaClient]): Contact ⇒ KademliaClient =
    client.service[KademliaClient](_)(sel)
}
