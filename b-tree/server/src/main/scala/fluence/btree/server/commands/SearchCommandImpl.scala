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
import fluence.btree.common.ValueRef
import fluence.btree.core.{Hash, Key}
import fluence.btree.protocol.BTreeRpc.SearchCallback
import fluence.btree.server.NodeId
import fluence.btree.server.core.{LeafNode, SearchCommand}

import scala.collection.Searching.SearchResult
import scala.language.higherKinds

/**
  * Command for searching some value in BTree (by client search key).
  * Search key is stored at the client. BTree server will never know search key.
  *
  * @param searchCallbacks A pack of functions that ask client to give some required details for the next step
  * @tparam F The type of effect, box for returning value
  */
case class SearchCommandImpl[F[_]](searchCallbacks: SearchCallback[F])(implicit ME: MonadError[F, Throwable])
    extends BaseSearchCommand[F](searchCallbacks) with SearchCommand[F, Key, ValueRef, NodeId] {

  override def submitLeaf(leaf: Option[LeafNode[Key, ValueRef, NodeId]]): F[SearchResult] = {
    val (keys, valuesChecksums) =
      leaf
        .map(l ⇒ l.keys → l.valuesChecksums)
        .getOrElse(Array.empty[Key] → Array.empty[Hash])

    searchCallbacks.submitLeaf(keys, valuesChecksums)
  }

}
