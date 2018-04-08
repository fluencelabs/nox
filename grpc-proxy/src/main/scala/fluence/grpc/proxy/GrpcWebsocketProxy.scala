package fluence.grpc.proxy

import cats.effect._
import cats.implicits._
import cats.~>
import fluence.proxy.grpc.WebsocketMessage
import fs2.StreamApp.ExitCode
import fs2.{io ⇒ _, _}
import org.http4s._
import org.http4s.dsl.Http4sDsl
import org.http4s.server.blaze.BlazeBuilder
import org.http4s.server.websocket._
import org.http4s.websocket.WebsocketBits._

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.Future
import scala.language.higherKinds

class GrpcWebsocketProxy[F[_]](inProcessGrpc: ProxyGrpc[F])(implicit F: Effect[F], runFuture: Future ~> F)
    extends StreamApp[F] with Http4sDsl[F] {

  def route(scheduler: Scheduler): HttpService[F] = HttpService[F] {

    case GET -> Root / "ws" ⇒
      val queue = async.unboundedQueue[F, WebSocketFrame]

      val echoReply: Pipe[F, WebSocketFrame, WebSocketFrame] = _.evalMap {
        case Binary(data, _) ⇒
          val a = for {
            message ← F.delay(WebsocketMessage.parseFrom(data))
            respF ← inProcessGrpc.handleMessage(message.service, message.method, message.protoMessage.newInput())
            resp ← runFuture(respF)
          } yield Binary(resp): WebSocketFrame
          a
        case _ ⇒
          val a = F.pure(Text("Unexpected message."): WebSocketFrame)
          a
      }

      val a = queue.flatMap { q ⇒
        val d = q.dequeue.through(echoReply)
        val e = q.enqueue
        WebSocketBuilder[F].build(d, e)
      }
      a
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
