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
import cats.syntax.compose._
import cats.syntax.profunctor._
import com.google.protobuf.ByteString
import fluence.codec.PureCodec
import fluence.kad.KeyProtobufCodecs._
import fluence.kad.protobuf.{NodesResponse, PingRequest}
import fluence.kad.protocol.{Contact, KademliaRpc, Key, Node}
import fluence.kad.{protobuf, protocol}
import fluence.stream.Connection
import fluence.transport.ProtobufCodec._

import scala.language.higherKinds

/**
 * Kademlia client.
 *
 */
class KademliaClient(connection: Connection)(
  implicit
  codec: PureCodec[protocol.Node[Contact], protobuf.Node]
) extends KademliaRpc[Contact] {

  private val service = "fluence.kad.protobuf.grpc.Kademlia"

  private val keyBS = PureCodec.codec[Key, ByteString].direct.toKleisli[IO]

  private val streamCodec = {
    import cats.instances.stream._
    PureCodec.codec[Stream[protocol.Node[Contact]], Stream[protobuf.Node]]
  }

  private val nodeContactCodec: PureCodec.Func[Array[Byte], Seq[Node[Contact]]] =
    protobufDynamicCodec(NodesResponse).rmap(_.nodes.toStream) andThen
      streamCodec.inverse.rmap(_.toSeq)

  private val pingCodec: fluence.codec.PureCodec.Func[Array[Byte], Node[Contact]] =
    protobufDynamicCodec(fluence.kad.protobuf.Node) andThen codec.inverse

  /**
   * Ping the contact, get its actual Node status, or fail.
   */
  override def ping(): IO[Node[Contact]] = {
    for {
      request ← generatedMessageCodec.runF[IO](PingRequest())
      responseBytes ← connection
        .handleUnary(service, "ping", request)
      response ← pingCodec.runF[IO](responseBytes)
    } yield response
  }

  /**
   * Perform a local lookup for a key, return K closest known nodes.
   *
   * @param key Key to lookup
   */
  override def lookup(key: Key, numberOfNodes: Int): IO[Seq[Node[Contact]]] = {
    for {
      k ← keyBS(key)
      request ← generatedMessageCodec.runF[IO](protobuf.LookupRequest(k, numberOfNodes))
      responseBytes ← connection.handleUnary(service, "lookup", request)
      res ← nodeContactCodec.runF[IO](responseBytes)
    } yield res
  }

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key.
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): IO[Seq[Node[Contact]]] = {
    for {
      k ← keyBS(key)
      moveAwayK ← keyBS(moveAwayFrom)
      req ← generatedMessageCodec.runF[IO](protobuf.LookupAwayRequest(k, moveAwayK, numberOfNodes))
      responseBytes ← connection.handleUnary(service, "lookupAway", req)
      res ← nodeContactCodec.runF[IO](responseBytes)
    } yield res
  }
}

object KademliaClient {
  def apply(streamHandler: Connection)(
    implicit
    codec: PureCodec[Node[Contact], protobuf.Node]
  ): KademliaRpc[Contact] = new KademliaClient(streamHandler)
}
