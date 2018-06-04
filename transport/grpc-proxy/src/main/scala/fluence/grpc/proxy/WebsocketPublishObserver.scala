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

package fluence.grpc.proxy

import fluence.proxy.grpc.WebsocketMessage
import fs2.async.mutable.Topic
import io.grpc.{StatusException, StatusRuntimeException}
import monix.eval.Task
import monix.execution.{Ack, Scheduler}
import monix.reactive.Observer
import org.http4s.websocket.WebsocketBits.{Binary, WebSocketFrame}

import scala.concurrent.Future

/**
 * Observer, that will publish every message or status changes to topic as WebsocketMessage.
 *
 * @param topic Publisher.
 * @param service Name of request's service.
 * @param method Name of request's method.
 * @param requestId Id of request.
 */
class WebsocketPublishObserver(topic: Topic[Task, WebSocketFrame], service: String, method: String, requestId: Long)(
  implicit scheduler: Scheduler
) extends Observer[WebSocketFrame] {

  private def genCompleteMessage(code: Int, description: String): WebsocketMessage = {
    val statusCode = fluence.proxy.grpc.Status.Code.fromValue(code)
    val status = fluence.proxy.grpc.Status(statusCode, description)
    WebsocketMessage(service, method, requestId, response = WebsocketMessage.Response.CompleteStatus(status))
  }

  override def onNext(elem: WebSocketFrame): Future[Ack] =
    topic.publish1(elem).map(_ ⇒ Ack.Continue).runAsync

  override def onError(ex: Throwable): Unit = {
    val errorMessage = ex match {
      case ex: StatusException ⇒
        val grpcStatus = ex.getStatus
        genCompleteMessage(grpcStatus.getCode.value(), grpcStatus.getDescription)
      case ex: StatusRuntimeException ⇒
        val grpcStatus = ex.getStatus
        genCompleteMessage(grpcStatus.getCode.value(), grpcStatus.getDescription)
      case ex: Throwable ⇒
        genCompleteMessage(fluence.proxy.grpc.Status.Code.INTERNAL.value, ex.getLocalizedMessage)
    }
    val frame = Binary(errorMessage.toByteArray)
    topic.publish1(frame)
    ex.printStackTrace()
  }

  override def onComplete(): Unit = {
    genCompleteMessage(fluence.proxy.grpc.Status.Code.OK.value, "")
  }
}
