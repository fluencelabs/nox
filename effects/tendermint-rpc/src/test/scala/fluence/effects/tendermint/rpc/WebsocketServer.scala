/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.effects.tendermint.rpc

import cats.Applicative
import cats.effect._
import cats.effect.concurrent.Ref
import cats.syntax.flatMap._
import cats.syntax.applicativeError._
import cats.syntax.functor._
import fluence.effects.{Backoff, WithCause}
import fs2.concurrent.{Queue, SignallingRef}
import fs2.{Pipe, Stream}
import org.http4s.{HttpRoutes, Response}
import org.http4s.dsl.Http4sDsl
import org.http4s.implicits._
import org.http4s.server.blaze.BlazeServerBuilder
import org.http4s.server.websocket.WebSocketBuilder
import org.http4s.websocket.WebSocketFrame
import org.http4s.websocket.WebSocketFrame.Close

import scala.language.higherKinds

case class WebsocketServer[F[_]: ConcurrentEffect: Timer](
  toClient: Queue[F, WebSocketFrame],
  fromClient: Queue[F, WebSocketFrame],
  signal: SignallingRef[F, Boolean]
) extends Http4sDsl[F] {

  private def routes(to: Stream[F, WebSocketFrame], from: Pipe[F, WebSocketFrame, Unit]): HttpRoutes[F] =
    HttpRoutes.of[F] {
      case GET -> Root / "websocket" =>
        WebSocketBuilder[F].build(to, from)
    }

  def requests(): Stream[F, WebSocketFrame] = fromClient.dequeue.interruptWhen(signal)
  def send(frame: WebSocketFrame): F[Unit] = toClient.enqueue1(frame)
  def close(): F[Unit] = toClient.enqueue1(Close()) >> signal.set(true)

  def start(port: Int): Stream[F, ExitCode] =
    for {
      exitCode <- Stream.eval(Ref[F].of(ExitCode.Success))
      server <- BlazeServerBuilder[F]
        .bindHttp(port)
        .withHttpApp(routes(toClient.dequeue, fromClient.enqueue).orNotFound)
        .serveWhile(signal, exitCode)
    } yield server
}

object WebsocketServer {

  def make[F[_]: Timer](port: Int)(implicit F: ConcurrentEffect[F]): Resource[F, WebsocketServer[F]] =
    Resource.make(
      for {
        to <- Queue.unbounded[F, WebSocketFrame]
        from <- Queue.unbounded[F, WebSocketFrame]
        signal <- SignallingRef[F, Boolean](false)
        server = WebsocketServer(to, from, signal)
        _ <- Concurrent[F].start(Backoff.default {
          server
            .start(port)
            .compile
            .drain
            .attemptT
            .leftMap(
              t =>
                new WithCause[Throwable] {
                  override def cause: Throwable = t
              }
            )
        })
      } yield server
    )(_.close())
}
