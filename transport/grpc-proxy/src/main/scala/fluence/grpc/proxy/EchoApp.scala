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
