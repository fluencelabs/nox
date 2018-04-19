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
import fs2.async.mutable.Queue
import fs2.interop.reactivestreams._
import fs2.{io ⇒ _, _}
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.http4s._
import org.http4s.dsl.Http4sDsl
import org.http4s.server.Server
import org.http4s.server.blaze.BlazeBuilder
import org.http4s.server.websocket._
import org.http4s.websocket.WebsocketBits.{WebSocketFrame, _}

import scala.language.higherKinds

/**
 * Websocket-to-grpc proxy server.
 */
object GrpcWebsocketProxy extends Http4sDsl[Task] {

  private def route(inProcessGrpc: InProcessGrpc, scheduler: Scheduler): HttpService[Task] = HttpService[Task] {

    case GET -> Root / "ws" ⇒
      //TODO add size of queue to config
      val queueF = async.boundedQueue[Task, WebSocketFrame](100)
      //Creates a proxy for each connection to separate the cache for all clients.
      val proxyGrpc = new ProxyGrpc(inProcessGrpc)

      val replyPipe: Pipe[Task, WebSocketFrame, WebSocketFrame] = _.flatMap {
        case Binary(data, _) ⇒
          val responseStream = for {
            message ← Task.eval(WebsocketMessage.parseFrom(data))
            response ← proxyGrpc
              .handleMessage(message.service, message.method, message.requestId, message.payload.newInput())
          } yield {
            response.map {
              Binary(_): WebSocketFrame
            }

          }

          Stream.eval(responseStream).flatMap(_.toReactivePublisher.toStream[Task]())
        case m ⇒
          Stream.eval(Task.pure(Text(s"Unexpected message: $m"): WebSocketFrame))
      }

      queueF.flatMap { queue: Queue[Task, WebSocketFrame] ⇒
        val dequeueStream = queue.dequeue.through(replyPipe)
        val enqueueStream = queue.enqueue
        WebSocketBuilder[Task].build(dequeueStream, enqueueStream)
      }
  }

  def startWebsocketServer(inProcessGrpc: InProcessGrpc, scheduler: Scheduler, port: Int): Task[Server[Task]] =
    for {

      server ← BlazeBuilder[Task]
        .bindHttp(port)
        .withWebSockets(true)
        .mountService(route(inProcessGrpc, scheduler))
        .start
    } yield server

}
