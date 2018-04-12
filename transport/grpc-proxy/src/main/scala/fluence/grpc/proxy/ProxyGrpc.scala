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
import io.grpc._
import io.grpc.internal.IoUtils
import io.grpc.stub.StreamObserver

import scala.concurrent.{ExecutionContext, Future, Promise}
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
      val call = inProcessGrpc.channel.newCall[Any, Any](methodDescriptor, CallOptions.DEFAULT)

      call.start(new ProxyListener[Any](onMessagePr), metadata)

      call.sendMessage(req)
      call.request(1)
      call.halfClose()

      onMessagePr.future
    }
    F.handleError(f) { e: Throwable ⇒
      onMessagePr.tryFailure(e)
      onMessagePr.future
    }
  }

  val bidiMap = Map.empty[Long, ClientCall[Any, Any]]

  def openBidiCall[Req, Resp](
    methodDescriptor: MethodDescriptor[Req, Resp],
    onMessagePr: Promise[Resp],
    onClosePr: Promise[(io.grpc.Status, Metadata)]
  ) = {
    val metadata = new Metadata()
    val call = inProcessGrpc.newCall[Req, Resp](methodDescriptor, CallOptions.DEFAULT)

    call.start(new ProxyListener[Resp](onMessagePr, onClosePr = Some(onClosePr)), metadata)

    call
  }

  private def bidiCall[Req, Resp](req: Req, methodDescriptor: MethodDescriptor[Req, Resp]): F[Future[Resp]] = {
    val onMessagePr = Promise[Resp]
    val onClosePr = Promise[(io.grpc.Status, Metadata)]
    val f = F.delay {
      val call = openBidiCall(methodDescriptor, onMessagePr, onClosePr)

      val a = StreamObserver[Resp]

      call.sendMessage(req)
      call.request(1)

      call.halfClose()

      //If onClose will be completed earlier, than onMessage, we will raise error

      runFuture(onClosePr.future)
      val raiseErrorOnClose: Future[Resp] = onClosePr.future.flatMap(
        _ ⇒ Future.failed(new RuntimeException("The call was completed before the message was received"))
      )

      Future.firstCompletedOf(Seq(onMessagePr.future, raiseErrorOnClose))
    }
    F.handleError(f) { e: Throwable ⇒
      onMessagePr.tryFailure(e)
      onMessagePr.future
    }
  }

  private def closeStream(streamId: Long) = {}

  /**
   * Handle proxying request for some service and method that registered in grpc server.
   *
   * @param service Name of grpc service (class name of service).
   * @param method Name of grpc method (method name of service).
   * @param request Input stream of bytes or close stream flag.
   *
   * @return Response as array of bytes.
   */
  def handleMessage(service: String, method: String, streamId: Long, request: Request): F[Response] = {
    for {
      methodDescriptor ← getMethodDescriptorF(service, method)
      _ = println("TYPE = " + methodDescriptor.getType)
      resp ← request match {
        case RequestInputStream(stream) ⇒
          for {
            request ← F.delay(methodDescriptor.parseRequest(stream))
            responseF ← unaryCall(request, methodDescriptor)
            response ← runFuture(responseF)
          } yield ResponseArrayByte(IoUtils.toByteArray(methodDescriptor.streamResponse(response))): Response
        case Close ⇒
          F.delay(NoResponse: Response)
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
