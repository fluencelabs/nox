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
import cats.implicits._
import fs2._
import fs2.StreamApp.ExitCode
import org.http4s._
import org.http4s.dsl.Http4sDsl
import org.http4s.server.blaze.BlazeBuilder
import org.http4s.server.websocket._
import org.http4s.websocket.WebsocketBits._

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.higherKinds

/**
 * Echo server for debugging websocket connections.
 */
//TODO remove this class when it's not needed
object EchoMain extends EchoApp[IO]

class EchoApp[F[_]](implicit F: Effect[F]) extends StreamApp[F] with Http4sDsl[F] {

  def route(scheduler: Scheduler): HttpService[F] = HttpService[F] {

    case GET -> Root / "wsecho" ⇒
      val queue = async.unboundedQueue[F, WebSocketFrame]
      val echoReply: Pipe[F, WebSocketFrame, WebSocketFrame] = _.collect {
        case Binary(msg, _) ⇒
          println("You sent to the server binary: " + msg.mkString(","))
          Binary(msg)
        case m ⇒
          Text("Unsupported.")
      }

      val toClient: Stream[F, WebSocketFrame] =
        scheduler.awakeEvery[F](1.seconds).map { d ⇒
          println("ping")
          Ping()
        }

      queue.flatMap { q ⇒
        val d = q.dequeue.through(echoReply) ++ toClient
        val e = q.enqueue
        WebSocketBuilder[F].build(d, e)
      }
  }

  def stream(args: List[String], requestShutdown: F[Unit]): Stream[F, ExitCode] =
    for {
      scheduler ← Scheduler[F](corePoolSize = 2)
      exitCode ← BlazeBuilder[F]
        .bindHttp(8080)
        .withWebSockets(true)
        .mountService(route(scheduler), "/http4s")
        .serve
    } yield exitCode

}
