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

import cats.Monad
import cats.data.EitherT
import cats.effect.IO
import fluence.codec.{CodecError, PureCodec}
import fluence.kad.protobuf.PingRequest
import fluence.kad.{protobuf, protocol}
import fluence.kad.protocol.{Contact, KademliaRpc, Key, Node}
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.WebsocketPipe.WebsocketClient
import fluence.transport.websocket.GrpcProxyClient
import monix.execution.Ack
import monix.execution.Ack.Stop

import scala.concurrent.{ExecutionContext, Future, Promise}
import monix.execution.Scheduler.Implicits.global
import monix.reactive.Observer

import scala.language.higherKinds
import scala.util.Random

class KademliaWebsocketClient(websocket: WebsocketClient[WebsocketMessage])(
  implicit
  codec: PureCodec[protocol.Node[Contact], protobuf.Node],
  ec: ExecutionContext
) extends KademliaRpc[Contact] {

  val service = "fluence.kad.protobuf.grpc.Kademlia"

  /**
   * Ping the contact, get its actual Node status, or fail.
   */
  override def ping(): IO[Node[Contact]] = {

    try {
      println(" START PINGING ===================")
      val requestId = Random.nextLong()

      val requestCodec = new PureCodec.Func[PingRequest, Array[Byte]] {
        override def apply[F[_]](input: PingRequest)(implicit F: Monad[F]): EitherT[F, CodecError, Array[Byte]] = {
          EitherT.liftF(F.pure(input.toByteString.toByteArray))
        }
      }

      val responseCodec1 = new PureCodec.Func[Array[Byte], fluence.kad.protobuf.Node] {
        override def apply[F[_]](
          input: Array[Byte]
        )(implicit F: Monad[F]): EitherT[F, CodecError, fluence.kad.protobuf.Node] = {
          EitherT.liftF(F.pure(fluence.kad.protobuf.Node.parseFrom(input)))
        }
      }

      val responseCodec2 = new PureCodec.Func[Array[Byte], protocol.Node[Contact]] {
        override def apply[F[_]](
          input: Array[Byte]
        )(implicit F: Monad[F]): EitherT[F, CodecError, protocol.Node[Contact]] = {
          val a = responseCodec1.unsafe(input)
          EitherT.liftF(F.pure(codec.inverse.unsafe(a)))
        }
      }

      val proxy = GrpcProxyClient.proxy(service, "ping", requestId, websocket, requestCodec, responseCodec2)

      val result: Promise[Node[Contact]] = Promise[protocol.Node[Contact]]

      val obs = new Observer[protocol.Node[Contact]] {

        override def onNext(elem: Node[Contact]): Future[Ack] = {
          result.success(elem)
          Future(Stop)
        }

        override def onError(ex: Throwable): Unit = {
          result.failure(ex)
          onComplete()
        }

        override def onComplete(): Unit = ()
      }

      proxy.input.onNext(PingRequest())

      proxy.output.subscribe(obs)

      IO.fromFuture(IO(result.future))
    } catch {
      case e: Throwable ⇒
        println("SOMETHING WRONG!!!")
        e.printStackTrace()
        throw e
    }
  }

  /**
   * Perform a local lookup for a key, return K closest known nodes.
   *
   * @param key Key to lookup
   */
  override def lookup(key: Key, numberOfNodes: Int): IO[Seq[Node[Contact]]] = ???

  /**
   * Perform a local lookup for a key, return K closest known nodes, going away from the second key.
   *
   * @param key Key to lookup
   */
  override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): IO[Seq[Node[Contact]]] = ???
}
