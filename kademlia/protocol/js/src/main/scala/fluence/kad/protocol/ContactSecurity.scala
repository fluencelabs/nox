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

import cats.Applicative
import cats.syntax.eq._

import scala.language.higherKinds

/**
 * ContactSecurity provides checks that particular Node[Contact] could be saved to a RoutingTable
 * JS version doesn't make use of java.net.InetAddress
 */
object ContactSecurity {

  private def checkMaybeLocal[F[_]: Applicative](self: Key): Node[Contact] ⇒ F[Boolean] =
    node ⇒ Applicative[F].pure(node.key =!= self && Key.checkPublicKey(node.key, node.contact.publicKey))

  def check[F[_]: Applicative](self: Key, acceptLocal: Boolean): Node[Contact] ⇒ F[Boolean] =
    checkMaybeLocal[F](self)
}
