package fluence.node.grpc

import cats.effect._
import cats.implicits._
import fs2.{io ⇒ _, _}
import fs2.StreamApp.ExitCode
import org.http4s._
import org.http4s.dsl.Http4sDsl
import org.http4s.server.blaze.BlazeBuilder
import org.http4s.server.websocket._
import org.http4s.websocket.WebsocketBits._

import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._

class GrpcWebsocketProxy[F[_]](inProcessGrpc: InProcessGrpc)(implicit F: Effect[F])
    extends StreamApp[F] with Http4sDsl[F] {

  def route(scheduler: Scheduler): HttpService[F] = HttpService[F] {
    case GET -> Root / "ws" ⇒
      val toClient: Stream[F, WebSocketFrame] =
        scheduler.awakeEvery[F](1.seconds).map(d ⇒ Text(s"Ping! $d"))
      val fromClient: Sink[F, WebSocketFrame] = _.evalMap { (ws: WebSocketFrame) ⇒
        ws match {
          case Text(t, _) ⇒ F.delay(println(t))
          case f ⇒ F.delay(println(s"Unknown type: $f"))
        }
      }
      WebSocketBuilder[F].build(toClient, fromClient)

    case GET -> Root / "wsecho" ⇒
      val queue = async.unboundedQueue[F, WebSocketFrame]
      val echoReply: Pipe[F, WebSocketFrame, WebSocketFrame] = _.collect {
        case Text(msg, _) ⇒ Text("You sent the server: " + msg)
        case _ ⇒ Text("Something new")
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
