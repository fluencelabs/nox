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

package fluence.btree.server.network.commands

import cats.MonadError
import cats.syntax.functor._
import fluence.btree.client.network._
import fluence.btree.client.{ Key, Value }
import fluence.btree.server.core.{ GetCommand, LeafNode }

/**
 * Command for searching some value in BTree (by client search key).
 * Search key is stored at the client. BTree server will never know search key.
 *
 * @param askRequiredDetails A function that ask client to give some required details for the next step
 *
 * @tparam F The type of effect, box for returning value
 */
class GetCommandImpl[F[_]](
    askRequiredDetails: (BTreeServerResponse) ⇒ F[Option[BTreeClientRequest]]
)(implicit ME: MonadError[F, Throwable]) extends BaseCommand[F](askRequiredDetails) with GetCommand[F, Key, Value] {

  override def submitLeaf(leaf: Option[LeafNode[Key, Value]]): F[Unit] = {
    val response =
      leaf.map(l ⇒ LeafResponse(l.keys, l.values))
        .getOrElse(LeafResponse(Array.empty[Key], Array.empty[Value]))

    askRequiredDetails(response)
      .map(_ ⇒ ())
  }

}
