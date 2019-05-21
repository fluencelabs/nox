package fluence.effects.tendermint.rpc

import cats.effect._
import cats.effect.concurrent.Ref
import cats.syntax.flatMap._
import cats.syntax.functor._
import fs2.concurrent.{Queue, SignallingRef}
import fs2.{Pipe, Stream}
import org.http4s.HttpRoutes
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

  def start(): Stream[F, ExitCode] =
    for {
      exitCode <- Stream.eval(Ref[F].of(ExitCode.Success))
      server <- BlazeServerBuilder[F]
        .bindHttp(8080)
        .withHttpApp(routes(toClient.dequeue, fromClient.enqueue).orNotFound)
        .serveWhile(signal, exitCode)
    } yield server
}

object WebsocketServer {

  def make[F[_]: ConcurrentEffect: Timer]: Resource[F, WebsocketServer[F]] = Resource.liftF(
    for {
      to <- Queue.unbounded[F, WebSocketFrame]
      from <- Queue.unbounded[F, WebSocketFrame]
      signal <- SignallingRef[F, Boolean](false)
      server = WebsocketServer(to, from, signal)
      _ <- Concurrent[F].start(server.start().compile.drain)
    } yield server
  )
}
