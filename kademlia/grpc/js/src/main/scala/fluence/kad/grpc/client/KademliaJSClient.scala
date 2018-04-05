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
import fluence.codec.PureCodec
import fluence.kad.grpc.JSCodecs._
import fluence.kad.grpc.KademliaGrpcService
import fluence.kad.protocol.{Contact, KademliaRpc, Key, Node}
import fluence.kad.{grpc, protocol}

import scala.concurrent.ExecutionContext
import scala.language.implicitConversions
import scala.scalajs.js.typedarray.Uint8Array

//TODO write integration test with kademlia network
//TODO merge it with KademliaJSClient in jvm project
/**
 * Implementation of KademliaClient over GRPC, with Task and Contact.
 *
 * @param stub GRPC Kademlia Stub
 */
class KademliaJSClient(stub: KademliaGrpcService)(
  implicit
  codec: PureCodec[protocol.Node[Contact], grpc.facade.Node],
  ec: ExecutionContext
) extends KademliaRpc[Contact] {

  private val keyUint = PureCodec.codec[Uint8Array, Key].inverse

  import cats.instances.stream._

  private val streamCodec = PureCodec.codec[Stream[protocol.Node[Contact]], Stream[grpc.facade.Node]]

  /**
   * Ping the contact, get its actual Node status, or fail
   *
   * @return
   */
  override def ping(): IO[Node[Contact]] =
    for {
      n ← IO.fromFuture(IO(stub.ping(new grpc.facade.PingRequest())))
      nc ← codec.inverse.runF[IO](n)
    } yield nc

  /**
   * Perform a local lookup for a key, return K closest known nodes
   *
   * @param key Key to lookup
   * @return
   */
  override def lookup(key: Key, numberOfNodes: Int): IO[Seq[Node[Contact]]] =
    for {
      k ← keyUint.runF[IO](key)
      res ← IO.fromFuture(IO(stub.lookup(grpc.facade.LookupRequest(k, numberOfNodes))))
      resDec ← streamCodec.inverse.runF[IO](res.nodes().toStream)
    } yield resDec

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): IO[Seq[Node[Contact]]] =
    for {
      k ← keyUint.runF[IO](key)
      moveAwayK ← keyUint.runF[IO](moveAwayFrom)
      res ← IO.fromFuture(
        IO(
          stub.lookupAway(grpc.facade.LookupAwayRequest(k, moveAwayK, numberOfNodes))
        )
      )
      resDec ← streamCodec.inverse.runF[IO](res.nodes().toStream)
    } yield resDec
}

object KademliaJSClient {

  /**
   * Shorthand to register KademliaClient inside NetworkClient.
   */
  def register(service: KademliaGrpcService)(
    implicit
    codec: PureCodec[protocol.Node[Contact], grpc.facade.Node],
    ec: ExecutionContext
  ): KademliaRpc[Contact] =
    new KademliaJSClient(service)

}
