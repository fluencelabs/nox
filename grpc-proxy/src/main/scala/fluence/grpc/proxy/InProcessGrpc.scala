package fluence.grpc.proxy

import cats.effect.Effect
import io.grpc.{ManagedChannel, ServerServiceDefinition}
import io.grpc.inprocess.{InProcessChannelBuilder, InProcessServerBuilder}

import scala.language.higherKinds

final case class InProcessGrpc(services: List[ServerServiceDefinition], channel: ManagedChannel)

object InProcessGrpc {

  def build[F[_]](name: String, services: List[ServerServiceDefinition])(implicit F: Effect[F]): F[InProcessGrpc] = {
    F.delay {
      val inProcessServer = InProcessServerBuilder.forName(name)
      services.foreach(s ⇒ inProcessServer.addService(s))
      inProcessServer.build().start()

      val channel = InProcessChannelBuilder.forName(name).build()

      InProcessGrpc(services, channel)
    }
  }
}
