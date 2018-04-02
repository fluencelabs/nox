package fluence.node.grpc

import io.grpc.{ServerMethodDefinition, ServerServiceDefinition}
import io.grpc.inprocess.{InProcessChannelBuilder, InProcessServerBuilder}

class InProcessGrpc(name: String, services: List[ServerServiceDefinition]) extends slogging.LazyLogging {

  val pp = InProcessServerBuilder.forName("in-process")
  services.foreach(s ⇒ pp.addService(s))
  pp.build().start()

  val chs = InProcessChannelBuilder.forName("in-process").build()

  def getMd[Req, Resp](service: String, method: String): Option[ServerMethodDefinition[Req, Resp]] = {
    for {
      sd ← services.find(_.getServiceDescriptor.getName == service)
      _ = logger.error("SERVICE DESCRIPTION == " + sd)
      m ← Option(sd.getMethod(service + "/" + method).asInstanceOf[ServerMethodDefinition[Req, Resp]])
    } yield m
  }

}
