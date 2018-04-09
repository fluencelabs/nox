package fluence.grpc.proxy

import io.grpc.{ManagedChannel, Server, ServerServiceDefinition}

import scala.collection.JavaConverters._
import scala.language.higherKinds

final case class InProcessGrpc(server: Server, channel: ManagedChannel) {
  val services: List[ServerServiceDefinition] = server.getServices.asScala.toList
}
