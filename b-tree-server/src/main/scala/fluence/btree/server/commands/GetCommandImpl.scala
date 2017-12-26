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

package fluence.btree.server.commands

import cats.MonadError
import fluence.btree.client.protocol.BTreeRpc.GetCallbacks
import fluence.btree.client.{ Key, Value }
import fluence.btree.server.core.{ GetCommand, LeafNode }

/**
 * Command for searching some value in BTree (by client search key).
 * Search key is stored at the client. BTree server will never know search key.
 *
 * @param getCallbacks A pack of functions that ask client to give some required details for the next step
 * @tparam F The type of effect, box for returning value
 */
class GetCommandImpl[F[_]](getCallbacks: GetCallbacks[F])(implicit ME: MonadError[F, Throwable])
  extends BaseSearchCommand[F](getCallbacks) with GetCommand[F, Key, Value] {

  override def submitLeaf(leaf: Option[LeafNode[Key, Value]]): F[Unit] = {
    val (keys, values) =
      leaf.map(l ⇒ l.keys → l.values)
        .getOrElse(Array.empty[Key] → Array.empty[Value])

    getCallbacks.submitLeaf(keys, values)
  }

}
