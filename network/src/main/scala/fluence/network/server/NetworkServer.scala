package fluence.network.server

import java.net.InetAddress

import fluence.network.Contact
import io.grpc.{ Server, ServerBuilder, ServerServiceDefinition }
import monix.eval.Task
import monix.execution.Scheduler.Implicits._

class NetworkServer private (
    server:   Server,
    contact:  Task[Contact],
    onShutdown: Task[Unit]
) {
  def start(): Unit = {
    server.start()
  }

  def shutdown(): Unit = {
    server.shutdown()
    onShutdown.runSyncMaybe
    server.awaitTermination()
  }


}

object NetworkServer {
  class Builder(val contact: Task[Contact], shutdown: Task[Unit], localPort: Int, services: List[ServerServiceDefinition]) {
    def add(service: ServerServiceDefinition): Builder =
      new Builder(contact, shutdown, localPort, service :: services)

    def build: NetworkServer =
      new NetworkServer(
        {
          val sb = ServerBuilder
            .forPort(localPort)
          services.foreach(sb.addService)
          sb.build()
        },
        contact,
        shutdown
      )
  }

  def builder(localPort: Int): Builder =
    new Builder(Task.now(Contact(InetAddress.getLocalHost, localPort)), Task.unit, localPort, Nil)

  def builder(localPort: Int, externalPort: Int, uPnP: UPnP = new UPnP()): Builder =
    new Builder(uPnP.addPort(externalPort, localPort).memoizeOnSuccess.onErrorRecover{
      case _ ⇒ Contact(InetAddress.getLocalHost, localPort)
    }, uPnP.deletePort(externalPort).memoizeOnSuccess, localPort, Nil)
}