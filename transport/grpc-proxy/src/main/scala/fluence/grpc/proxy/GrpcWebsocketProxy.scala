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

import com.google.protobuf.ByteString
import fluence.grpc.{InProcessGrpc, ProxyWebsocketGrpc, Result}
import fluence.proxy.grpc.WebsocketMessage
import fs2.async.mutable.Topic
import fs2.{io ⇒ _, _}
import monix.eval.Task
import monix.execution.{Scheduler ⇒ TaskScheduler}
import org.http4s._
import org.http4s.dsl.Http4sDsl
import org.http4s.server.Server
import org.http4s.server.blaze.BlazeBuilder
import org.http4s.server.websocket._
import org.http4s.websocket.WebsocketBits.{WebSocketFrame, _}

import scala.concurrent.duration._
import scala.language.higherKinds

/**
 * Websocket-to-grpc proxy server.
 */
object GrpcWebsocketProxy extends Http4sDsl[Task] with slogging.LazyLogging {

  private def route(
    inProcessGrpc: InProcessGrpc,
    scheduler: Scheduler,
    pingInterval: FiniteDuration = 2.seconds
  )(implicit taskScheduler: TaskScheduler): HttpService[Task] = HttpService[Task] {

    case GET -> Root ⇒
      //Creates a proxy for each connection to separate the cache for all clients.
      val proxyGrpc = new ProxyWebsocketGrpc(inProcessGrpc)

      def replyPipe(topic: Topic[Task, WebSocketFrame]): Sink[Task, WebSocketFrame] = _.flatMap {
        case Binary(data, _) ⇒
          val handleRequest = for {
            message ← Task(WebsocketMessage.parseFrom(data))
            _ = logger.debug(s"Handle websocket message $message")
            responseObservableOp ← proxyGrpc
              .handleMessage(
                message.service,
                message.method,
                message.requestId,
                RequestConverter.toEither(message.response)
              )
          } yield {
            responseObservableOp.map {
              case Result(responseObservable, controlOnComplete) ⇒
                val binaryObservable = responseObservable.map { bytes ⇒
                  val responseMessage =
                    message.copy(response = WebsocketMessage.Response.Payload(ByteString.copyFrom(bytes)))
                  Binary(responseMessage.toByteArray)
                }

                //splitting observer and topic for asynchronous request handling
                val obs = new WebsocketPublishObserver(
                  topic,
                  message.service,
                  message.method,
                  message.requestId,
                  controlOnComplete
                )

                binaryObservable.subscribe(obs)
            }
          }

          Stream.eval(handleRequest.map(_ ⇒ ()))
        case m ⇒
          logger.warn(s"Unexpected message in GrpcWebsocketProxy: $m")
          Stream.eval(Task.pure(()))
      }

      for {
        topic ← async.topic[Task, WebSocketFrame](Ping())
        _ = scheduler.awakeEvery[Task](pingInterval).map(d ⇒ topic.publish1(Ping()))
        // publish to topic from websocket and send to websocket from topic publisher
        // TODO add maxQueued to config
        ws ← WebSocketBuilder[Task].build(topic.subscribe(1000), replyPipe(topic))
      } yield {
        ws
      }
  }

  def startWebsocketServer(
    inProcessGrpc: InProcessGrpc,
    scheduler: Scheduler,
    port: Int
  )(implicit taskScheduler: TaskScheduler): Task[Server[Task]] =
    for {
      server ← BlazeBuilder[Task]
        .bindHttp(port, "0.0.0.0")
        .withWebSockets(true)
        .mountService(route(inProcessGrpc, scheduler))
        .start
    } yield server

}
