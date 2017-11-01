package fluence.network.server

import java.net.InetAddress

import fluence.network.Contact
import monix.eval.Task
import org.bitlet.weupnp.{ GatewayDevice, GatewayDiscover }
import org.slf4j.LoggerFactory

import scala.collection.JavaConverters._
import scala.concurrent.duration.Duration

// TODO: find a way to cover with tests: mock external upnp, check our logic
/**
 * UPnP port forwarding utility
 * @param clientName Client name to register on gateway
 * @param protocol Protocol name to register on gateway
 * @param httpReadTimeout Http read timeout
 * @param discoverTimeout Devices discovery timeout
 */
class UPnP(
    clientName:      String   = "Fluence",
    protocol:        String   = "TCP",
    httpReadTimeout: Duration = Duration.Undefined,
    discoverTimeout: Duration = Duration.Undefined
) {

  private val log = LoggerFactory.getLogger(getClass)

  /**
   * Memoized gateway task
   */
  val gateway: Task[GatewayDevice] = Task {
    log.info("Going to discover GatewayDevice...")
    GatewayDevice.setHttpReadTimeout(
      Option(httpReadTimeout).filter(_.isFinite()).map(_.toMillis.toInt).getOrElse(GatewayDevice.getHttpReadTimeout)
    )

    val discover = new GatewayDiscover()
    discover.setTimeout(
      Option(discoverTimeout).filter(_.isFinite()).map(_.toMillis.toInt).getOrElse(GatewayDevice.getHttpReadTimeout)
    )

    discover
  }.flatMap {
    discover ⇒
      Task(
        Option(discover.discover).map(_.asScala).map(_.toMap).getOrElse(Map())
      ).flatMap[GatewayDiscover] { gatewayMap ⇒
          if (gatewayMap.isEmpty) {
            log.warn("Gateway map is empty")
            Task.raiseError[GatewayDiscover](new NoSuchElementException("Gateway map is empty"))
          } else Task.now(discover)
        }
  }.flatMap { discover ⇒
    Option(discover.getValidGateway) match {
      case None ⇒
        log.warn("There is no connected UPnP gateway device")
        Task.raiseError[GatewayDevice](new NoSuchElementException("There is no connected UPnP gateway device"))

      case Some(device) ⇒
        log.info("Found device: " + device)
        Task.now(device)
    }
  }.memoizeOnSuccess

  /**
   * Memoized external address
   */
  val externalAddress: Task[InetAddress] =
    gateway
      .flatMap(gw ⇒ Task(gw.getExternalIPAddress))
      .map(InetAddress.getByName)
      .map{ addr ⇒
        log.info("External IP address: {}", addr.getHostAddress)
        addr
      }
      .memoizeOnSuccess

  /**
   * Add external port on gateway or fail with exception
   * @param externalPort External port to forward to local port
   * @param localPort To handle requests
   * @return Contact with external address
   */
  def addPort(externalPort: Int, localPort: Int): Task[Contact] =
    gateway.flatMap { gw ⇒
      log.info("Going to add port mapping: {} => {}", externalPort, localPort)
      Task(
        gw.addPortMapping(externalPort, localPort, gw.getLocalAddress.getHostAddress, protocol, clientName)
      ).flatMap {
          case true ⇒
            log.info("External port successfully mapped")
            externalAddress.map(addr ⇒ Contact(addr, externalPort))

          case false ⇒
            log.warn("Can't add port mapping")
            Task.raiseError(new RuntimeException("Can't add port mapping"))
        }
    }

  /**
   * Remove external port mapping on gateway
   * @param externalPort External port with registered mapping
   * @return
   */
  def deletePort(externalPort: Int): Task[Unit] =
    gateway.flatMap(gw ⇒ Task(gw.deletePortMapping(externalPort, protocol)))

}
