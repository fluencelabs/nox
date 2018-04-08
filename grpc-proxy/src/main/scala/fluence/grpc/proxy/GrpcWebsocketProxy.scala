package fluence.grpc.proxy

import cats.effect._
import cats.implicits._
import fluence.proxy.grpc.WebsocketMessage
import fs2.StreamApp.ExitCode
import fs2.{io ⇒ _, _}
import org.http4s._
import org.http4s.dsl.Http4sDsl
import org.http4s.server.blaze.BlazeBuilder
import org.http4s.server.websocket._
import org.http4s.websocket.WebsocketBits._

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.higherKinds

/**
 * Websocket-to-grpc proxy app
 * @param proxyGrpc Proxy grpc API
 */
class GrpcWebsocketProxy[F[_]](proxyGrpc: ProxyGrpc[F])(implicit F: Effect[F]) extends StreamApp[F] with Http4sDsl[F] {

  def route(scheduler: Scheduler): HttpService[F] = HttpService[F] {

    case GET -> Root / "ws" ⇒
      val queue = async.unboundedQueue[F, WebSocketFrame]

      val echoReply: Pipe[F, WebSocketFrame, WebSocketFrame] = _.evalMap {
        case Binary(data, _) ⇒
          for {
            message ← F.delay(WebsocketMessage.parseFrom(data))
            response ← proxyGrpc.handleMessage(message.service, message.method, message.protoMessage.newInput())
          } yield Binary(response): WebSocketFrame
        case _ ⇒
          F.pure(Text("Unexpected message."): WebSocketFrame)
      }

      queue.flatMap { q ⇒
        val d = q.dequeue.through(echoReply)
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
