package fluence.network

import java.net.InetAddress

import cats.Show
import monix.eval.Coeval

case class Contact(
    ip: InetAddress,
    port: Int
) {
  lazy val b64seed: String =
    new String(java.util.Base64.getEncoder.encode(Array.concat(ip.getAddress, {
      val bb = java.nio.ByteBuffer.allocate(Integer.BYTES)
      bb.putInt(port)
      bb.array()
    })))

  lazy val isLocal: Boolean =
    ip.isLoopbackAddress ||
      ip.isAnyLocalAddress ||
      ip.isLinkLocalAddress ||
      ip.isSiteLocalAddress ||
      ip.isMCSiteLocal ||
      ip.isMCGlobal ||
      ip.isMCLinkLocal ||
      ip.isMCNodeLocal ||
      ip.isMCOrgLocal
}

object Contact {
  implicit val show: Show[Contact] =
    (c: Contact) ⇒ s"${c.ip.getHostAddress}:${c.port}(${c.b64seed})"

  def readB64seed(str: String): Coeval[Contact] = Coeval{
    val bytes = java.util.Base64.getDecoder.decode(str)
    val addr = InetAddress.getByAddress(bytes.take(4))
    val port = java.nio.ByteBuffer.wrap(bytes.drop(4)).getInt
    Contact(addr, port)
  }
}
