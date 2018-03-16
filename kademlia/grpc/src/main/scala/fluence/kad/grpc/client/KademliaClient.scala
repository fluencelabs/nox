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

import cats.data.{ EitherT, Kleisli }
import cats.effect.Async
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.~>
import com.google.protobuf.ByteString
import fluence.codec.Codec
import fluence.kad.grpc.NodesResponse
import fluence.kad.protocol.{ Contact, KademliaRpc, Key, Node }
import fluence.kad.{ grpc, protocol }
import fluence.transport.grpc.GrpcCodecs._
import fluence.transport.grpc.client.GrpcRunner
import io.grpc.{ CallOptions, ManagedChannel }

import scala.concurrent.Future
import scala.language.{ higherKinds, implicitConversions, reflectiveCalls }

/**
 * Implementation of KademliaClient over GRPC, with Task and Contact.
 *
 * @param stub GRPC Kademlia Stub
 */
class KademliaClient[F[_] : Async, E](stub: grpc.KademliaGrpc.KademliaStub)(implicit
    run: GrpcRunner.Run[F, E],
    codec: Codec[F, protocol.Node[Contact], grpc.Node],
    errorCodec: Codec[F, E, grpc.Error]) extends KademliaRpc[F, Contact] {

  override type Error = E

  private val keyBS = Codec.codec[F, ByteString, Key].inverse

  import cats.instances.stream._

  private val streamCodec = Codec.codec[F, Stream[protocol.Node[Contact]], Stream[grpc.Node]]

  private val streamEitherK = Kleisli[F, NodesResponse.Response, Either[E, Seq[protocol.Node[Contact]]]]{
    case resp if resp.isError ⇒
      errorCodec.decode(resp.error.get)
        .map(Left(_))

    case resp if resp.isNodes ⇒
      streamCodec
        .decode(resp.nodes.get.nodes.toStream)
        .map(Right(_))

    case _ ⇒
      Async[F].raiseError(new IllegalArgumentException("Response message is empty")) // TODO: it also should be an error of type E
  }

  /**
   * Ping the contact, get its actual Node status, or fail
   *
   * @return
   */
  override def ping(): EitherT[F, E, Node[Contact]] =
    run(stub.ping(grpc.PingRequest())).map(_.response).flatMapF[E, Node[Contact]]{
      case resp if resp.isError ⇒
        errorCodec.decode(resp.error.get).map(Left(_))

      case resp if resp.isNode ⇒
        codec.decode(resp.node.get).map(Right(_))

      case _ ⇒
        Async[F].raiseError(new IllegalArgumentException("Response message is empty")) // TODO: it also should be an error of type E
    }

  /**
   * Perform a local lookup for a key, return K closest known nodes
   *
   * @param key Key to lookup
   * @return
   */
  override def lookup(key: Key, numberOfNodes: Int): EitherT[F, E, Seq[Node[Contact]]] =
    EitherT(for {
      k ← keyBS(key)
      res ← run(stub.lookup(grpc.LookupRequest(k, numberOfNodes)))
        .map(_.response)
        .flatMapF(streamEitherK.run)
        .value
    } yield res)

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): EitherT[F, E, Seq[Node[Contact]]] =
    EitherT(for {
      k ← keyBS(key)
      moveAwayK ← keyBS(moveAwayFrom)
      res ← run(
        stub.lookupAway(grpc.LookupAwayRequest(k, moveAwayK, numberOfNodes))
      )
        .map(_.response)
        .flatMapF(streamEitherK.run)
        .value
    } yield res)
}

object KademliaClient {
  /**
   * Shorthand to register KademliaClient inside NetworkClient.
   *
   * @param channel     Channel to remote node
   * @param callOptions Call options
   */
  def register[F[_] : Async, E]()(
    channel: ManagedChannel,
    callOptions: CallOptions
  )(implicit
    run: GrpcRunner.Run[F, E],
    codec: Codec[F, protocol.Node[Contact], grpc.Node],
    errorCodec: Codec[F, E, grpc.Error]
  ): KademliaRpc.Aux[F, Contact, E] =
    new KademliaClient[F, E](new grpc.KademliaGrpc.KademliaStub(channel, callOptions))

}
