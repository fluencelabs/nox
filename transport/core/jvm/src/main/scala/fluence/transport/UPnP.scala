package fluence.transport

import java.net.InetAddress

import cats.effect.IO
import org.bitlet.weupnp.{ GatewayDevice, GatewayDiscover }

import scala.concurrent.duration.Duration
import scala.collection.JavaConverters._

/**
 * UPnP port forwarding utility
 * @param clientName Client name to register on gateway
 * @param protocol Protocol name to register on gateway
 * @param gateway UPnP gateway
 * @param externalAddress This node's external address
 */
class UPnP(
    clientName: String,
    protocol: String,
    gateway: GatewayDevice,
    val externalAddress: InetAddress
) extends slogging.LazyLogging {

  /**
   * Add external port on gateway or fail with exception
   * @param localPort Port on local interface
   * @param externalPort External port to forward to local port
   * @return Either successful IO or an error
   */
  def addPort(externalPort: Int, localPort: Int): IO[Unit] =
    IO {
      logger.info("Going to add port mapping: {} => {}", externalPort, localPort)
      gateway.addPortMapping(externalPort, localPort, gateway.getLocalAddress.getHostAddress, protocol, clientName)
    }.flatMap {
      case true ⇒
        logger.info("External port successfully mapped")
        IO.unit

      case false ⇒
        logger.warn("Can't add port mapping")
        IO.raiseError(new RuntimeException("Can't add port mapping"))
    }

  /**
   * Remove external port mapping on gateway
   * @param externalPort External port with registered mapping
   * @return
   */
  def deletePort(externalPort: Int): IO[Unit] =
    IO(gateway.deletePortMapping(externalPort, protocol))

}

object UPnP extends slogging.LazyLogging {
  /**
   * Builds an UPnP instance.
   * Notice that executing it produces effects of changing [[GatewayDevice]]'s timeouts
   * and looking up [[GatewayDevice]] in the network.
   *
   * @param clientName Client name to register on gateway
   * @param protocol Protocol name to register on gateway (TCP or UDP)
   * @param httpReadTimeout Http Read Timeout for gateway requests
   * @param discoverTimeout Timeout to lookup gateway in local network
   */
  def apply(
    clientName: String = "Fluence",
    protocol: String = "TCP",
    httpReadTimeout: Duration = Duration.Undefined,
    discoverTimeout: Duration = Duration.Undefined
  ): IO[UPnP] =
    for {
      gateway ← discoverGateway(httpReadTimeout, discoverTimeout)
      externalAddress ← getExternalAddress(gateway)
    } yield new UPnP(clientName, protocol, gateway, externalAddress)

  private def discoverGateway(httpReadTimeout: Duration, discoverTimeout: Duration): IO[GatewayDevice] = IO {
    // Effect is changing timeouts in global GatewayDevice state
    logger.info("Going to discover GatewayDevice...")
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
      IO(
        // This is a blocking effect
        Option(discover.discover).map(_.asScala).map(_.toMap).getOrElse(Map())
      ).flatMap[GatewayDiscover] { gatewayMap ⇒
          if (gatewayMap.isEmpty) {
            logger.warn("Gateway map is empty")
            IO.raiseError[GatewayDiscover](new NoSuchElementException("Gateway map is empty"))
          } else IO.pure(discover)
        }
  }.flatMap { discover ⇒
    // There effects are exceptions from getValidGateway, or null return
    Option(discover.getValidGateway) match {
      case None ⇒
        logger.warn("There is no connected UPnP gateway device")
        IO.raiseError[GatewayDevice](new NoSuchElementException("There is no connected UPnP gateway device"))

      case Some(device) ⇒
        logger.info("Found device: " + device)
        IO.pure(device)
    }
  }

  private def getExternalAddress(gateway: GatewayDevice): IO[InetAddress] =
    IO(gateway.getExternalIPAddress)
      .map(InetAddress.getByName)
      .map{ addr ⇒
        logger.info("External IP address: {}", addr.getHostAddress)
        addr
      }

}
