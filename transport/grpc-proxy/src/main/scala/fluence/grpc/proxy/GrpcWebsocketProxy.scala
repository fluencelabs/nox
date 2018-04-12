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
import fs2.{io ⇒ _, _}
import org.http4s._
import org.http4s.dsl.Http4sDsl
import org.http4s.server.blaze.BlazeBuilder
import org.http4s.server.websocket._
import org.http4s.websocket.WebsocketBits
import org.http4s.websocket.WebsocketBits.{WebSocketFrame, _}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.higherKinds

/**
 * Websocket-to-grpc proxy standalone app.
 * TODO make it pluggable and abstract from Stream
 * @param proxyGrpc Proxy grpc API.
 */
class GrpcWebsocketProxy[F[_]](proxyGrpc: ProxyGrpc[F], port: Int = 8080)(implicit F: Effect[F])
    extends StreamApp[F] with Http4sDsl[F] {

  def route(scheduler: Scheduler): HttpService[F] = HttpService[F] {

    case GET -> Root / "ws" ⇒
      //TODO add size of queue to config
      val queueF = async.boundedQueue[F, WebSocketFrame](100)

      val echoReply: Pipe[F, WebSocketFrame, WebSocketFrame] = _.evalMap {
        case Binary(data, _) ⇒
          for {
            message ← F.delay(WebsocketMessage.parseFrom(data))
            response ← proxyGrpc
              .handleMessage(message.service, message.method, message.streamId, ProxyGrpc.replyConverter(message.reply))
          } yield {
            response match {
              case ResponseArrayByte(bytes) ⇒ Binary(bytes): WebSocketFrame
              case NoResponse ⇒ WebsocketBits.Close(): WebSocketFrame
            }

          }
        case m ⇒
          F.pure(Text(s"Unexpected message: $m"): WebSocketFrame)
      }

      queueF.flatMap { queue ⇒
        val d = queue.dequeue.through(echoReply)
        val e = queue.enqueue
        WebSocketBuilder[F].build(d, e)
      }
  }

  override def stream(args: List[String], requestShutdown: F[Unit]): Stream[F, ExitCode] =
    for {
      scheduler ← Scheduler[F](corePoolSize = 2)
      exitCode ← BlazeBuilder[F]
        .bindHttp(port)
        .withWebSockets(true)
        .mountService(route(scheduler))
        .serve
    } yield exitCode

}
