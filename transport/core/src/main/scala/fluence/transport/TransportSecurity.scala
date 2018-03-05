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

package fluence.transport

import java.net.InetAddress

import cats.Applicative
import cats.syntax.applicative._
import cats.syntax.eq._
import fluence.kad.protocol.{ Contact, Key, Node }

import scala.language.higherKinds

object TransportSecurity {

  /**
   * Checks if a node can be saved to RoutingTable.
   * TODO: return the reason in Either, why exactly can't node be saved
   *
   * @param self This node's Key
   * @param acceptLocal If true, local addresses will be accepted; should be used only in testing
   * @tparam F Effect
   * @return Function to be called for each node prior to updating RoutingTable; returns F[true] if checks passed
   */
  def canBeSaved[F[_] : Applicative](self: Key, acceptLocal: Boolean): Node[Contact] ⇒ F[Boolean] =
    node ⇒ {
      if (node.key === self || !Key.checkPublicKey(node.key, node.contact.publicKey)) false.pure[F]
      else if (acceptLocal) true.pure[F]
      else try {
        val ip = InetAddress.getByName(node.contact.addr)
        val isLocal = ip.isLoopbackAddress ||
          ip.isAnyLocalAddress ||
          ip.isLinkLocalAddress ||
          ip.isSiteLocalAddress ||
          ip.isMCSiteLocal ||
          ip.isMCGlobal ||
          ip.isMCLinkLocal ||
          ip.isMCNodeLocal ||
          ip.isMCOrgLocal
        (!isLocal).pure[F]
      } catch {
        case _: Throwable ⇒ false.pure[F] // TODO: return in Either
      }
    }

}
