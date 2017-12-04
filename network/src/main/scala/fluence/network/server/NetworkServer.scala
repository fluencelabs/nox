package fluence.network.server

import java.net.InetAddress

import com.google.protobuf.ByteString
import fluence.kad.Key
import fluence.network.proto.kademlia.{ Header, Node }
import fluence.network.Contact
import io.grpc.{ Server, ServerBuilder, ServerServiceDefinition }
import monix.eval.Task
import monix.execution.Scheduler.Implicits._

import scala.concurrent.Await
import scala.concurrent.duration.FiniteDuration

/**
 * Server wrapper
 * @param server grpc Server instance
 * @param contact Self-discovered contact of this server
 * @param onShutdown Callback to launch on shutdown
 */
class NetworkServer private (
    server: Server,
    val contact: Task[Contact],
    onShutdown: Task[Unit]
) {
  /**
   * Launch server, grab ports, or fail
   */
  def start(): Task[Unit] =
    Task(server.start())

  /**
   * Shut the server down, release ports
   */
  def shutdown(timeout: FiniteDuration): Unit = {
    server.shutdown()
    Await.ready(onShutdown.runAsync, timeout)
    server.awaitTermination()
  }

}

object NetworkServer {

  /**
   * Builder for grpc network server
   * @param contact Self-discovered contact for current node
   * @param shutdown What to call back on shutdown
   * @param localPort Local port to launch server on
   * @param services GRPC services definitions
   */
  class Builder(val contact: Task[Contact], shutdown: Task[Unit], localPort: Int, services: List[ServerServiceDefinition]) {
    /**
     * Add new grpc service to the server
     * @param service Service definition
     * @return
     */
    def add(service: ServerServiceDefinition): Builder =
      new Builder(contact, shutdown, localPort, service :: services)

    /**
     * The header to be sent with each outgoing request
     * @param key Node key
     * @param advertize Whether to advertize this node with requests or not
     */
    def header(key: Key, advertize: Boolean = true): Task[Header] =
      contact.map(c ⇒
        Header(
          from = Some(
            Node(
              id = ByteString.copyFrom(key.id),
              ip = ByteString.copyFrom(c.ip.getAddress),
              port = c.port
            )
          ),
          advertize = true
        ))

    /**
     * Build grpc server with all the defined services
     * @return
     */
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

  /**
   * Builder for a local port, no upnp
   * @param localPort Local port
   * @return
   */
  def builder(localPort: Int): Builder =
    new Builder(Task.now(Contact(InetAddress.getLocalHost, localPort)), Task.unit, localPort, Nil)

  /**
   * Builder for a local port, with upnp used to provide external port
   * @param localPort Local port
   * @param externalPort External port to be grabbed on gateway device
   * @param uPnP UPnP instance to use for gateway management
   * @return
   */
  def builder(localPort: Int, externalPort: Int, uPnP: UPnP = new UPnP()): Builder =
    new Builder(uPnP.addPort(externalPort, localPort).memoizeOnSuccess.onErrorRecover{
      case _ ⇒ Contact(InetAddress.getLocalHost, localPort)
    }, uPnP.deletePort(externalPort).memoizeOnSuccess, localPort, Nil)

  /**
   * Builder for config object
   * @param conf Server config object
   * @return
   */
  def builder(conf: ServerConf): Builder =
    conf.externalPort.fold(builder(conf.localPort))(ext ⇒ builder(conf.localPort, ext))

  /**
   * Builder for default config object, read from typesafe conf
   * @return
   */
  def builder: Builder =
    builder(ServerConf.read())
}
