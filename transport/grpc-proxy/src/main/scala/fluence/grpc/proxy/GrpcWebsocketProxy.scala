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

import cats.effect._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.proxy.grpc.WebsocketMessage
import fs2.StreamApp.ExitCode
import fs2.async.mutable.{Queue, Topic}
import fs2.{io ⇒ _, _}
import fs2._
import monix.eval.Task
import org.http4s._
import org.http4s.dsl.Http4sDsl
import org.http4s.server.blaze.BlazeBuilder
import org.http4s.server.websocket._
import org.http4s.websocket.WebsocketBits
import org.http4s.websocket.WebsocketBits.{WebSocketFrame, _}
import org.reactivestreams.Publisher
import fs2.interop.reactivestreams._

import scala.concurrent.ExecutionContext
import monix.execution.Scheduler.Implicits.global
import scala.language.higherKinds

/**
 * Websocket-to-grpc proxy standalone app.
 * TODO make it pluggable and abstract from Stream
 * @param proxyGrpc Proxy grpc API.
 */
class GrpcWebsocketProxy(proxyGrpc: ProxyGrpc, port: Int = 8080) extends StreamApp[Task] with Http4sDsl[Task] {

  def route(scheduler: Scheduler): HttpService[Task] = HttpService[Task] {

    case GET -> Root / "ws" ⇒
      //TODO add size of queue to config
      val queueF = async.boundedQueue[Task, WebSocketFrame](100)

      val echoReply: Pipe[Task, WebSocketFrame, WebSocketFrame] = _.flatMap {
        case Binary(data, _) ⇒
          val a = for {
            message ← Task.eval(WebsocketMessage.parseFrom(data))
            response ← proxyGrpc
              .handleMessage(message.service, message.method, message.streamId, message.payload.newInput())
          } yield {
            response.map {
              case ResponseArrayByte(bytes) ⇒ Binary(bytes): WebSocketFrame
            }.take(1)

          }

          Stream.eval(a).flatMap(a ⇒ a.toReactivePublisher.toStream[Task]())
        case m ⇒
          Stream.eval(Task.pure(Text(s"Unexpected message: $m"): WebSocketFrame))
      }

      queueF.flatMap { queue: Queue[Task, WebSocketFrame] ⇒
        val dequeueStream = queue.dequeue.through(echoReply)
        val enqueueStream = queue.enqueue
        WebSocketBuilder[Task].build(dequeueStream, enqueueStream)
      }
  }

  override def stream(args: List[String], requestShutdown: Task[Unit]): Stream[Task, ExitCode] =
    for {
      scheduler ← Scheduler[Task](corePoolSize = 2)
      exitCode ← BlazeBuilder[Task]
        .bindHttp(port)
        .withWebSockets(true)
        .mountService(route(scheduler))
        .serve
    } yield exitCode

}
