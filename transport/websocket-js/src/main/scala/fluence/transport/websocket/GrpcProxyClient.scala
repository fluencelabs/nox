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

import java.nio.ByteBuffer

import com.google.protobuf.ByteString
import fluence.codec
import fluence.codec.{Codec, PureCodec}
import fluence.proxy.grpc.WebsocketMessage
import fluence.transport.websocket.WebsocketPipe.WebsocketClient
import monix.execution.Ack
import monix.reactive.{Observable, Observer}

import scala.concurrent.Future

object GrpcProxyClient {

  private def message(service: String, method: String, requestId: Long)(payload: Array[Byte]): WebsocketMessage = {
    WebsocketMessage(service, method, requestId, ByteString.copyFrom(payload))
  }

  def proxy[A, B](
    service: String,
    method: String,
    requestId: Long,
    websocketClient: WebsocketClient[WebsocketMessage],
    requestCodec: PureCodec.Func[A, Array[Byte]],
    responseCodec: PureCodec.Func[Array[Byte], B]
  ): WebsocketPipe[A, B] = {
    def messageCr: Array[Byte] ⇒ WebsocketMessage = message(service, method, requestId)

    val wsObserver = websocketClient.input
    val wsObservable = websocketClient.output

    val tObserver: Observer[A] = new Observer[A] {
      override def onNext(elem: A): Future[Ack] = {
        println("SENDING IN PROXY === " + elem)
        val message = messageCr(requestCodec.unsafe(elem))
        println("MESSAGE === " + message)
        wsObserver.onNext(message)
      }

      override def onError(ex: Throwable): Unit = wsObserver.onError(ex)

      override def onComplete(): Unit = wsObserver.onComplete()
    }

    val tObservable = wsObservable.collect {
      case mes @ WebsocketMessage(s, m, rId, payload) if s == service && m == method && rId == rId ⇒
        println("MESSAGE IN OBSERVABLE === " + mes)
        val resp = responseCodec.unsafe(payload.toByteArray)
        resp
    }

    WebsocketPipe(tObserver, tObservable, websocketClient.statusOutput)
  }
}
