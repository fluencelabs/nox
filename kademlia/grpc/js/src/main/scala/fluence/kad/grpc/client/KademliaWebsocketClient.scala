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
import com.google.protobuf.ByteString
import fluence.codec.{CodecError, PureCodec}
import fluence.kad.protobuf.PingRequest
import fluence.kad.{protobuf, protocol}
import fluence.kad.protocol.{Contact, KademliaRpc, Key, Node}
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.WebsocketPipe.WebsocketClient
import fluence.transport.websocket.{GrpcProxyClient, WebsocketPipe}
import monix.execution.Ack
import monix.execution.Ack.Stop

import scala.concurrent.{ExecutionContext, Future, Promise}
import monix.execution.Scheduler.Implicits.global
import monix.reactive.{Observable, Observer}
import fluence.kad.KeyProtobufCodecs._
import scalapb.GeneratedMessage

import scala.language.higherKinds
import scala.util.Random

class KademliaWebsocketClient(websocket: WebsocketClient[WebsocketMessage])(
  implicit
  codec: PureCodec[protocol.Node[Contact], protobuf.Node],
  ec: ExecutionContext
) extends KademliaRpc[Contact] {

  val service = "fluence.kad.protobuf.grpc.Kademlia"

  private val keyBS = PureCodec.codec[Key, ByteString].direct.toKleisli[IO]

  private val streamCodec = {
    import cats.instances.stream._
    PureCodec.codec[Stream[protocol.Node[Contact]], Stream[protobuf.Node]]
  }

  def requestAndWaitOneResult[A, B](request: A, pipe: WebsocketPipe[A, B]): Future[B] = {
    val result: Promise[B] = Promise[B]

    val obs = new Observer[B] {

      override def onNext(elem: B): Future[Ack] = {
        result.success(elem)
        Future(Stop)
      }

      override def onError(ex: Throwable): Unit = {
        result.failure(ex)
        onComplete()
      }

      override def onComplete(): Unit = ()
    }

    pipe.output.subscribe(obs)

    pipe.input.onNext(request)

    result.future
  }

  val requestCodec = new PureCodec.Func[GeneratedMessage, Array[Byte]] {
    override def apply[F[_]](input: GeneratedMessage)(implicit F: Monad[F]): EitherT[F, CodecError, Array[Byte]] = {
      EitherT.liftF(F.pure(input.toByteString.toByteArray))
    }
  }

  /**
   * Ping the contact, get its actual Node status, or fail.
   */
  override def ping(): IO[Node[Contact]] = {

    try {
      println(" START PINGING ===================")

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

      val proxy: WebsocketPipe[GeneratedMessage, Node[Contact]] =
        GrpcProxyClient.proxy(service, "ping", websocket, requestCodec, responseCodec2)

      IO.fromFuture(IO(requestAndWaitOneResult(PingRequest(), proxy)))
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
  override def lookup(key: Key, numberOfNodes: Int): IO[Seq[Node[Contact]]] = {
    for {
      k ← keyBS(key)
      request = protobuf.LookupRequest(k, numberOfNodes)
      responseCodec1 = new PureCodec.Func[Array[Byte], fluence.kad.protobuf.NodesResponse] {
        override def apply[F[_]](
          input: Array[Byte]
        )(implicit F: Monad[F]): EitherT[F, CodecError, fluence.kad.protobuf.NodesResponse] = {
          EitherT.liftF(F.pure(fluence.kad.protobuf.NodesResponse.parseFrom(input)))
        }
      }
      responseCodec = new PureCodec.Func[Array[Byte], Seq[protocol.Node[Contact]]] {
        override def apply[F[_]](
          input: Array[Byte]
        )(implicit F: Monad[F]): EitherT[F, CodecError, Seq[protocol.Node[Contact]]] = {
          val res = responseCodec1.unsafe(input)
          streamCodec.inverse(res.nodes.toStream).map(_.toSeq)
        }
      }
      proxy = GrpcProxyClient.proxy(service, "lookup", websocket, requestCodec, responseCodec)
      res ← IO.fromFuture(IO(requestAndWaitOneResult(request, proxy)))
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
      req = protobuf.LookupAwayRequest(k, moveAwayK, numberOfNodes)
      responseCodec1 = new PureCodec.Func[Array[Byte], fluence.kad.protobuf.NodesResponse] {
        override def apply[F[_]](
          input: Array[Byte]
        )(implicit F: Monad[F]): EitherT[F, CodecError, fluence.kad.protobuf.NodesResponse] = {
          EitherT.liftF(F.pure(fluence.kad.protobuf.NodesResponse.parseFrom(input)))
        }
      }
      responseCodec = new PureCodec.Func[Array[Byte], Seq[protocol.Node[Contact]]] {
        override def apply[F[_]](
          input: Array[Byte]
        )(implicit F: Monad[F]): EitherT[F, CodecError, Seq[protocol.Node[Contact]]] = {
          val res = responseCodec1.unsafe(input)
          streamCodec.inverse(res.nodes.toStream).map(_.toSeq)
        }
      }
      proxy = GrpcProxyClient.proxy(service, "lookupAway", websocket, requestCodec, responseCodec)
      res ← IO.fromFuture(IO(requestAndWaitOneResult(req, proxy)))
    } yield res
  }
}
