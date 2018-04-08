package fluence.grpc.proxy

import java.io.InputStream

import cats.effect.Effect
import cats.syntax.flatMap._
import cats.syntax.functor._
import io.grpc._
import io.grpc.internal.IoUtils

import scala.concurrent.{Future, Promise}
import scala.language.higherKinds
import scala.concurrent.ExecutionContext.Implicits.global

class ProxyGrpc[F[_]](services: List[ServerServiceDefinition], ch: ManagedChannel)(
  implicit F: Effect[F]
) extends slogging.LazyLogging {

  def getMethodDescriptor[Req, Resp](service: String, method: String): Option[MethodDescriptor[Req, Resp]] = {
    for {
      sd ← services.find(_.getServiceDescriptor.getName == service)
      m ← Option(sd.getMethod(service + "/" + method).asInstanceOf[ServerMethodDefinition[Req, Resp]])
      descriptor ← Option(m.getMethodDescriptor)
    } yield descriptor
  }

  def getMethodDescriptorF[Req, Resp](service: String, method: String): F[MethodDescriptor[Req, Resp]] = {
    F.delay(getMethodDescriptor[Req, Resp](service, method)).flatMap {
      case Some(md) ⇒ F.pure(md)
      case None ⇒ F.raiseError(new IllegalArgumentException(s"There is no $service/$method method."))
    }
  }

  def unaryCall[Req, Resp](req: Req, methodDescriptor: MethodDescriptor[Req, Resp]): F[Future[Resp]] = {
    F.delay {
      val onMessagePr = Promise[Resp]

      val metadata = new Metadata()
      val call = ch.newCall[Req, Resp](methodDescriptor, CallOptions.DEFAULT)

      call.start(new ProxyListener[Resp](onMessagePr), metadata)

      call.sendMessage(req)
      call.request(1)
      call.halfClose()

      onMessagePr.future
    }
  }

  def handleMessage(service: String, method: String, request: InputStream): F[Future[Array[Byte]]] = {

    for {
      methodDescriptor ← getMethodDescriptorF[Any, Any](service, method)
      request ← F.delay(methodDescriptor.parseRequest(request))
      response ← unaryCall(request, methodDescriptor)
      streamResponse ← F.delay(response.map { r ⇒
        IoUtils.toByteArray(methodDescriptor.streamResponse(r))
      })
    } yield {
      streamResponse
    }
  }

}
