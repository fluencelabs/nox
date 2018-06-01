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
import fluence.proxy.grpc.WebsocketMessage
import fs2.async.mutable.Topic
import fs2.{io ⇒ _, _}
import io.grpc.StatusException
import monix.eval.Task
import monix.execution.Ack
import monix.execution.Scheduler.Implicits.global
import monix.reactive.Observer
import org.http4s._
import org.http4s.dsl.Http4sDsl
import org.http4s.server.Server
import org.http4s.server.blaze.BlazeBuilder
import org.http4s.server.websocket._
import org.http4s.websocket.WebsocketBits.{WebSocketFrame, _}

import scala.concurrent.Future
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
  ): HttpService[Task] = HttpService[Task] {

    case GET -> Root ⇒
      //Creates a proxy for each connection to separate the cache for all clients.
      val proxyGrpc = new ProxyGrpc(inProcessGrpc)

      def genCompleteMessage(message: WebsocketMessage, code: Int, description: String) = {
        val statusCode = fluence.proxy.grpc.Status.Code.fromValue(code)
        val status = fluence.proxy.grpc.Status(statusCode, description)
        message.copy(response = WebsocketMessage.Response.CompleteStatus(status))
      }

      def replyPipe(topic: Topic[Task, WebSocketFrame]): Sink[Task, WebSocketFrame] = _.flatMap {
        case Binary(data, _) ⇒
          val responseStream = for {
            message ← Task(WebsocketMessage.parseFrom(data))
            _ = logger.debug(s"Handle websocket message $message")
            // TODO message.response.payload.get.newInput() rewrite with error handling
            responseObservable ← proxyGrpc
              .handleMessage(
                message.service,
                message.method,
                message.requestId,
                message.response.payload.get.newInput()
              )
            binaryObservable = responseObservable.map { bytes ⇒
              val responseMessage =
                message.copy(response = WebsocketMessage.Response.Payload(ByteString.copyFrom(bytes)))
              Binary(responseMessage.toByteArray)
            }
          } yield {
            //splitting observer and topic for asynchronous request handling
            val obs = new Observer[WebSocketFrame] {
              override def onNext(elem: WebSocketFrame): Future[Ack] =
                topic.publish1(elem).map(_ ⇒ Ack.Continue).runAsync

              override def onError(ex: Throwable): Unit = {
                val errorMessage = ex match {
                  case ex: StatusException ⇒
                    val grpcStatus = ex.getStatus
                    genCompleteMessage(message, grpcStatus.getCode.value(), grpcStatus.getDescription)
                  case ex: Throwable ⇒
                    genCompleteMessage(message, fluence.proxy.grpc.Status.Code.INTERNAL.value, ex.getLocalizedMessage)
                }
                val frame = Binary(errorMessage.toByteArray)
                topic.publish1(frame)
                ex.printStackTrace()
              }

              override def onComplete(): Unit = {
                genCompleteMessage(message, fluence.proxy.grpc.Status.Code.OK.value, "")
              }
            }

            binaryObservable.subscribe(obs)
          }

          Stream.eval(responseStream.map(_ ⇒ ()))
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

  def startWebsocketServer(inProcessGrpc: InProcessGrpc, scheduler: Scheduler, port: Int): Task[Server[Task]] =
    for {

      server ← BlazeBuilder[Task]
        .bindHttp(port, "0.0.0.0")
        .withWebSockets(true)
        .mountService(route(inProcessGrpc, scheduler))
        .start
    } yield server

}
