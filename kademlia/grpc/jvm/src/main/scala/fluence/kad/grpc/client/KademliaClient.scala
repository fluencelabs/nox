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

import cats.effect.IO
import com.google.protobuf.ByteString
import fluence.codec.PureCodec
import fluence.kad.protocol.{Contact, KademliaRpc, Key, Node}
import fluence.kad.{grpc, protocol}
import io.grpc.{CallOptions, ManagedChannel}
import fluence.transport.grpc.KeyProtobufCodecs._

import scala.concurrent.ExecutionContext
import scala.language.implicitConversions

/**
 * Implementation of KademliaClient over GRPC, with Task and Contact.
 *
 * @param stub GRPC Kademlia Stub
 */
class KademliaClient(stub: grpc.KademliaGrpc.Kademlia)(
  implicit
  codec: PureCodec[protocol.Node[Contact], grpc.Node],
  ec: ExecutionContext
) extends KademliaRpc[Contact] {

  private val keyBS = PureCodec.codec[Key, ByteString].direct.toKleisli[IO]

  import cats.instances.stream._

  private val streamCodec = PureCodec.codec[Stream[protocol.Node[Contact]], Stream[grpc.Node]]

  /**
   * Ping the contact, get its actual Node status, or fail
   */
  override def ping(): IO[Node[Contact]] =
    for {
      n ← IO.fromFuture(IO(stub.ping(grpc.PingRequest())))
      nc ← codec.inverse.runF[IO](n)
    } yield nc

  /**
   * Perform a local lookup for a key, return K closest known nodes
   *
   * @param key Key to lookup
   */
  override def lookup(key: Key, numberOfNodes: Int): IO[Seq[Node[Contact]]] =
    for {
      k ← keyBS(key)
      res ← IO.fromFuture(IO(stub.lookup(grpc.LookupRequest(k, numberOfNodes))))
      resDec ← streamCodec.inverse.runF[IO](res.nodes.toStream)
    } yield resDec

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): IO[Seq[Node[Contact]]] =
    for {
      k ← keyBS(key)
      moveAwayK ← keyBS(moveAwayFrom)
      res ← IO.fromFuture(
        IO(
          stub.lookupAway(grpc.LookupAwayRequest(k, moveAwayK, numberOfNodes))
        )
      )
      resDec ← streamCodec.inverse.runF[IO](res.nodes.toStream)
    } yield resDec
}

object KademliaClient {

  /**
   * Shorthand to register KademliaClient inside NetworkClient.
   *
   * @param channel     Channel to remote node
   * @param callOptions Call options
   */
  def register()(
    channel: ManagedChannel,
    callOptions: CallOptions
  )(
    implicit
    codec: PureCodec[protocol.Node[Contact], grpc.Node],
    ec: ExecutionContext
  ): KademliaRpc[Contact] =
    new KademliaClient(new grpc.KademliaGrpc.KademliaStub(channel, callOptions))

}
