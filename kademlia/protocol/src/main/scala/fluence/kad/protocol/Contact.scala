/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.kad.protocol

import java.net.InetAddress

import cats.{ Eval, Show }

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

  def readB64seed(str: String): Eval[Contact] = Eval.later{
    val bytes = java.util.Base64.getDecoder.decode(str)
    val addr = InetAddress.getByAddress(bytes.take(4))
    val port = java.nio.ByteBuffer.wrap(bytes.drop(4)).getInt
    Contact(addr, port)
  }
}
