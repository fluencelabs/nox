package fluence.node.grpc

import java.io.InputStream

import cats.effect.{Effect, IO}
import io.grpc._
import io.grpc.inprocess.{InProcessChannelBuilder, InProcessServerBuilder}
import io.grpc.internal.IoUtils
import cats.syntax.functor._
import cats.syntax.flatMap._

import scala.concurrent.{Await, Promise}
import scala.concurrent.duration._
import scala.language.higherKinds

class ProxyGrpc[F[_]](services: List[ServerServiceDefinition], ch: ManagedChannel)(implicit F: Effect[F])
    extends slogging.LazyLogging {

  def getMethodDescriptor[Req, Resp](service: String, method: String): Option[MethodDescriptor[Req, Resp]] = {
    for {
      sd ← services.find(_.getServiceDescriptor.getName == service)
      _ = logger.error("SERVICE DESCRIPTION == " + sd)
      m ← Option(sd.getMethod(service + "/" + method).asInstanceOf[ServerMethodDefinition[Req, Resp]])
      descriptor ← Option(m.getMethodDescriptor)
    } yield descriptor
  }

  def getMethodDescriptorF[Req, Resp](service: String, method: String): F[MethodDescriptor[Req, Resp]] = {
    F.delay(getMethodDescriptor[Req, Resp](service, method)).flatMap {
      case Some(md) ⇒ F.pure(md)
      case None ⇒ F.raiseError(new RuntimeException(""))
    }
  }

  def unaryCall[Req, Resp](req: Req, methodDescriptor: MethodDescriptor[Req, Resp]) = {
    val onMessagePr = Promise[Resp]

    val metadata = new Metadata()
    val call = ch.newCall[Req, Resp](methodDescriptor, CallOptions.DEFAULT)

    call.start(new ProxyListener[Resp](onMessagePr), metadata)

    call.sendMessage(req)
    call.request(1)
    call.halfClose()

    onMessagePr.future
  }

  def handleMessage(service: String, method: String, request: InputStream): F[Array[Byte]] = {

    for {
      methodDescriptor ← getMethodDescriptorF[Any, Any](service, method)
      req ← F.delay(methodDescriptor.parseRequest(request))
    } yield {
      val res = Await.result(unaryCall(req, methodDescriptor), 10.seconds)

      val streamResp = methodDescriptor.streamResponse(res)
      IoUtils.toByteArray(streamResp)
    }
  }

}
