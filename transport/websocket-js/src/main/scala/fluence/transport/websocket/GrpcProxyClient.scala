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

package fluence.transport.websocket

import cats.effect.IO
import com.google.protobuf.ByteString
import fluence.codec.PureCodec
import fluence.proxy.grpc.Status.Code
import fluence.proxy.grpc.{Status, WebsocketMessage}
import monix.execution.Ack
import monix.execution.Ack.Continue
import monix.reactive.{Observable, Observer}

import scala.concurrent.{ExecutionContext, Future}
import scala.language.higherKinds
import scala.util.Random

/**
 * @param connection Websocket transport layer.
 */
class GrpcProxyClient(connection: IO[WebsocketPipe[WebsocketMessage, WebsocketMessage]]) {

  private def message(service: String, method: String, requestId: Long)(payload: Array[Byte]): WebsocketMessage = {
    WebsocketMessage(service, method, requestId, WebsocketMessage.Response.Payload(ByteString.copyFrom(payload)))
  }

  /**
   * Create proxy WebsocketPipe for working with GRPC interfaces on server.
   * @param service Name of GRPC service.
   * @param method Name of GRPC method.
   * @param requestCodec Codec for converting requests to byte array.
   * @param responseCodec Codec for converting responses from byte array.
   * @tparam A Request type.
   * @tparam B Response type.
   * @return Pipe with input observer, output observable and websocket status observable.
   */
  def proxy[A, B](
    service: String,
    method: String,
    requestCodec: PureCodec.Func[A, Array[Byte]],
    responseCodec: PureCodec.Func[Array[Byte], B]
  )(implicit ec: ExecutionContext): IO[WebsocketPipe[A, B]] = {
    connection.map { websocketClient ⇒
      val requestId = Random.nextLong()
      def messageCreation: Array[Byte] ⇒ WebsocketMessage = message(service, method, requestId)

      val wsObserver = websocketClient.input
      val wsObservable = websocketClient.output

      val proxyObserver: Observer[A] = new Observer[A] {
        override def onNext(elem: A): Future[Ack] = {
          val req = requestCodec.unsafe(elem)
          val message = messageCreation(req)
          wsObserver.onNext(message)
          // this will break backpressure, but if we do the chain of futures,
          // logic will be broken due to incomprehensible circumstances
          // TODO investigate and fix it
          Future(Continue)
        }

        override def onError(ex: Throwable): Unit = wsObserver.onError(ex)

        override def onComplete(): Unit = wsObserver.onComplete()
      }

      //we will collect only messages that have equals method name, service name and request id
      val proxyObservable = wsObservable.collect {
        case WebsocketMessage(s, m, rId, payload) if s == service && m == method && rId == rId ⇒
          payload
      }.takeWhile {
        case WebsocketMessage.Response.CompleteStatus(status) if status.code == Code.OK ⇒
          false
        case _ ⇒ true
      }.flatMap {
        case WebsocketMessage.Response.Payload(payload) ⇒
          Observable(responseCodec.unsafe(payload.toByteArray))
        case WebsocketMessage.Response.CompleteStatus(status) if status.code == Code.OK ⇒
          Observable()
        case WebsocketMessage.Response.CompleteStatus(status) ⇒
          Observable.raiseError(new StatusException(status))
        case WebsocketMessage.Response.Empty ⇒
          Observable.raiseError(new StatusException(Status(Status.Code.UNKNOWN, "Empty response received.")))
      }

      WebsocketPipe(proxyObserver, proxyObservable, websocketClient.statusOutput)
    }
  }
}
