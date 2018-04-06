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

import cats.{Applicative, Id}
import cats.syntax.eq._
import cats.syntax.applicative._
import cats.syntax.apply._

import scala.language.higherKinds

/**
 * ContactSecurity provides checks that particular Node[Contact] could be saved to a RoutingTable
 */
object ContactSecurity {

  /**
   * Performs cryptographical checks against Node[Contact]: public key conforms Kademlia key, and it's not this node
   * @param self Current node's Key
   * @tparam F Effect
   * @return Whether the node can be saved or not
   */
  private def cryptoCheck[F[_]: Applicative](self: Key): Node[Contact] ⇒ F[Boolean] =
    node ⇒
      Applicative[F].pure(node.key =!= self && Key.checkPublicKey[Id](node.key, node.contact.publicKey).value.isRight)

  /**
   * Performs a check for Node's address, trying to prove that it's not a loopback
   * @tparam F Effect
   * @return True if address seems to be non'local
   */
  private def nonLocalCheck[F[_]: Applicative]: Node[Contact] ⇒ F[Boolean] =
    node ⇒
      try {
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

  /**
   * Checks if Node can be saved to a local RoutingTable
   *
   * @param self Current node's Kademlia Key
   * @param acceptLocal Whether it is possible to accept local contacts with loopback addresses or not
   * @tparam F Effect
   * @return True if node can be saved
   */
  def check[F[_]: Applicative](self: Key, acceptLocal: Boolean): Node[Contact] ⇒ F[Boolean] =
    if (acceptLocal) cryptoCheck[F](self)
    else
      node ⇒
        (
          cryptoCheck[F](self).apply(node),
          nonLocalCheck[F].apply(node)
        ).mapN(_ && _)
}
