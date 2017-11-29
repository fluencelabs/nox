package fluence.network

import java.net.InetAddress

import cats.Show

case class Contact(
    ip: InetAddress,
    port: Int
)

object Contact {
  implicit val show: Show[Contact] =
    c ⇒ s"${c.ip.getHostAddress}:${c.port}"
}
