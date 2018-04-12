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

import java.io.InputStream

import cats.effect.Effect
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.~>
import fluence.proxy.grpc.WebsocketMessage
import fluence.proxy.grpc.WebsocketMessage.Reply.ProtoMessage
import io.grpc.MethodDescriptor.MethodType
import io.grpc._
import io.grpc.internal.IoUtils
import monix.reactive.{MulticastStrategy, Observable, OverflowStrategy}

import scala.concurrent.{ExecutionContext, Future, Promise}
import monix.execution.Scheduler.Implicits.global

import scala.collection.mutable
import scala.language.higherKinds

/**
 * Service to proxy requests in grpc from another source.
 *
 * @param inProcessGrpc In-process services and client channel.
 */
class ProxyGrpc[F[_]](inProcessGrpc: InProcessGrpc)(
  implicit F: Effect[F],
  runFuture: Future ~> F,
  ec: ExecutionContext
) extends slogging.LazyLogging {

  /**
   * Get grpc method descriptor from registered services.
   *
   * @param service Name of service.
   * @param method Name of method.
   *
   * @return Method descriptor or None, if there is no descriptor in registered services.
   */
  private def getMethodDescriptor(service: String, method: String): Option[MethodDescriptor[Any, Any]] = {
    for {
      serviceDescriptor ← inProcessGrpc.services.find(_.getServiceDescriptor.getName == service)
      serverMethodDefinition ← Option(
        serviceDescriptor.getMethod(service + "/" + method).asInstanceOf[ServerMethodDefinition[Any, Any]]
      )
      methodDescriptor ← Option(serverMethodDefinition.getMethodDescriptor)
    } yield methodDescriptor
  }

  private def getMethodDescriptorF(service: String, method: String): F[MethodDescriptor[Any, Any]] = {
    F.delay(getMethodDescriptor(service, method)).flatMap {
      case Some(md) ⇒ F.pure(md)
      case None ⇒ F.raiseError(new IllegalArgumentException(s"There is no $service/$method method."))
    }
  }

  /**
   * Unary call to grpc service.
   *
   */
  private def unaryCall(req: Any, methodDescriptor: MethodDescriptor[Any, Any]): F[Future[Any]] = {
    val onMessagePr = Promise[Any]
    val onClosePr = Promise[(io.grpc.Status, Metadata)]
    val f = F.delay {
      val metadata = new Metadata()
      val call = inProcessGrpc.newCall[Any, Any](methodDescriptor, CallOptions.DEFAULT)

      call.start(new ProxyListener[Any](onMessagePr, onClosePr), metadata)

      call.sendMessage(req)
      call.request(1)
      call.halfClose()

      runFuture(onClosePr.future)
      val raiseErrorOnClose: Future[Any] = onClosePr.future.flatMap(
        _ ⇒ Future.failed(new RuntimeException("The call was completed before the message was received"))
      )

      Future.firstCompletedOf(Seq(onMessagePr.future, raiseErrorOnClose))
    }
    F.handleError(f) { e: Throwable ⇒
      onMessagePr.tryFailure(e)
      onMessagePr.future
    }
  }

  val bidiMap = mutable.Map.empty[Long, (ClientCall[Any, Any], Observable[Any])]

  private val overflow: OverflowStrategy.Synchronous[Nothing] = OverflowStrategy.Unbounded

  def openBidiCall[Req, Resp](
    methodDescriptor: MethodDescriptor[Req, Resp]
  ): (ClientCall[Req, Resp], Observable[Response]) = {
    val metadata = new Metadata()
    val call = inProcessGrpc.newCall[Req, Resp](methodDescriptor, CallOptions.DEFAULT)

    val (in, out) = Observable.multicast[Resp](MulticastStrategy.replay, overflow)

    call.start(new StreamProxyListener[Resp](in), metadata)

    val mappedOut = out.collect {
      case r ⇒
        println("IN MAPPED === " + r)
        ResponseArrayByte(IoUtils.toByteArray(methodDescriptor.streamResponse(r))): Response
    }

    (call, mappedOut)
  }

  /**
   * Handle proxying request for some service and method that registered in grpc server.
   *
   * @param service Name of grpc service (class name of service).
   * @param method Name of grpc method (method name of service).
   * @param request Input stream of bytes or close stream flag.
   *
   * @return Response as array of bytes.
   */
  def handleMessage(service: String, method: String, streamId: Long, request: Request): F[Observable[Response]] = {
    for {
      methodDescriptor ← getMethodDescriptorF(service, method)
      _ = println("TYPE = " + methodDescriptor.getType)
      resp ← request match {
        case RequestInputStream(stream) ⇒
          for {
            call ← F.delay(bidiMap.get(streamId))
            o ← call match {
              case Some((c, obs)) ⇒
                println("DO SOME CALL")
                F.delay {
                  val req = methodDescriptor.parseRequest(stream)
                  c.sendMessage(req)
                  c.request(1)
                  val dropped = obs.foreachL { r ⇒
                    println("DROP === " + r)
                  }
                  Observable(NoResponse: Response)

                }
              case None ⇒
                for {
                  req ← F.delay(methodDescriptor.parseRequest(stream))
                  (c, obs) = openBidiCall[Any, Any](methodDescriptor)
                  _ = if (methodDescriptor.getType != MethodType.UNARY) bidiMap.put(streamId, (c, obs))
                } yield {
                  println("SEND MESSAGE == " + request)
                  c.sendMessage(req)
                  println("sending")
                  c.request(1)

                  println("request")
//                  obs.doOnNext(r ⇒ "callback on resp === " + r)

                  if (methodDescriptor.getType == MethodType.UNARY) c.halfClose()
                  obs
                }
            }
          } yield o
        case Close ⇒
          println("CLOSING === ")
          bidiMap.get(streamId).foreach {
            case (c, o) ⇒
              c.halfClose()
          }
          bidiMap.remove(streamId)
          F.delay(Observable(NoResponse: Response))
      }

    } yield resp
  }

}

sealed trait Request
final case class RequestInputStream(stream: InputStream) extends Request
object Close extends Request

sealed trait Response
final case class ResponseArrayByte(bytes: Array[Byte]) extends Response
object NoResponse extends Response

object ProxyGrpc {

  def replyConverter(reply: WebsocketMessage.Reply): Request = {
    reply match {
      case ProtoMessage(value) ⇒ RequestInputStream(value.newInput())
      case WebsocketMessage.Reply.Close(_) ⇒ Close
    }
  }
}
