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
import fluence.proxy.grpc.Status.Code
import fluence.proxy.grpc.{Status, WebsocketMessage}
import fluence.stream.Connection
import monix.execution.Scheduler
import monix.reactive.Observable

import scala.language.higherKinds
import scala.util.Random

class WebsocketConnection(
  //TODO change WebsocketPipe to something more generalized in order to use different websocket clients
  connection: IO[WebsocketPipe[WebsocketMessage, WebsocketMessage]]
)(
  implicit s: Scheduler
) extends Connection {

  private def message(service: String, method: String, requestId: Long)(payload: Array[Byte]): WebsocketMessage = {
    WebsocketMessage(service, method, requestId, WebsocketMessage.Response.Payload(ByteString.copyFrom(payload)))
  }

  override def handle(
    service: String,
    method: String,
    requests: Observable[Array[Byte]]
  ): IO[Observable[Array[Byte]]] = {
    connection.map { websocketClient ⇒
      val requestId = Random.nextLong()
      def messageCreation: Array[Byte] ⇒ WebsocketMessage = message(service, method, requestId)

      val wsObserver = websocketClient.input
      val wsObservable = websocketClient.output

      requests.map(messageCreation).subscribe(wsObserver)

      //we will collect only messages that have equals method name, service name and request id
      val proxyObservable = wsObservable.collect {
        case WebsocketMessage(srv, m, rId, payload) if srv == service && m == method && rId == rId ⇒
          payload
      }.takeWhile {
        case WebsocketMessage.Response.CompleteStatus(status) if status.code == Code.OK ⇒
          false
        case _ ⇒ true
      }.flatMap {
        case WebsocketMessage.Response.Payload(payload) ⇒
          Observable(payload.toByteArray)
        case WebsocketMessage.Response.CompleteStatus(status) if status.code == Code.OK ⇒
          Observable()
        case WebsocketMessage.Response.CompleteStatus(status) ⇒
          Observable.raiseError(new StatusException(status))
        case WebsocketMessage.Response.Empty ⇒
          Observable.raiseError(new StatusException(Status(Status.Code.UNKNOWN, "Empty response received.")))
      }

      proxyObservable
    }
  }

  override def handleUnary(service: String, method: String, request: Array[Byte]): IO[Array[Byte]] = {
    for {
      observable ← handle(service, method, Observable(request))
      res ← observable.headL.toIO
    } yield {
      res
    }
  }
}
