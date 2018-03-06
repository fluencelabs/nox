package fluence.transport

import java.net.InetAddress

import cats.effect.IO
import org.bitlet.weupnp.GatewayDevice

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
