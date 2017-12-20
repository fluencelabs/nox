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

package fluence.btree.server

import cats.Show
import cats.syntax.show._
import fluence.btree.server.core.PutDetails

/**
 * ''Show'' type class implementations for [[MerkleBTree]] entities.
 */
object MerkleBTreeShow {

  implicit val showBytes: Show[Array[Byte]] =
    (b: Array[Byte]) ⇒ new String(b)

  implicit def showBytes(implicit sb: Show[Array[Byte]]): Show[Array[Array[Byte]]] =
    Show.show((array: Array[Array[Byte]]) ⇒ array.map(sb.show).mkString("[", ",", "]"))

  implicit def showLongArray: Show[Array[Long]] =
    Show.show((array: Array[Long]) ⇒ array.mkString("[", ",", "]"))

  implicit def showBranch(implicit sb: Show[Array[Array[Byte]]], sl: Show[Array[Long]]): Show[Branch] = {
    Show.show((t: Branch) ⇒ s"""Branch(${sb.show(t.keys)}, ${sl.show(t.children)}, ${t.size}, ${t.checksum.show})""")
  }

  implicit def showLeaf(implicit sb: Show[Array[Array[Byte]]], sl: Show[Array[Long]]): Show[Leaf] = {
    Show.show((l: Leaf) ⇒ s"""Leaf(${sb.show(l.keys)}, ${sb.show(l.values)}, ${l.size}, ${l.checksum.show})""")
  }

  implicit def showPutDetails(implicit sb: Show[Array[Byte]]): Show[PutDetails] = {
    Show.show((pd: PutDetails) ⇒ s"""PutDetails(${sb.show(pd.key)}, ${sb.show(pd.value)}, ${pd.searchResult})""")
  }

  implicit def showNode(implicit st: Show[Branch], sl: Show[Leaf]): Show[Node] = {
    case t: Branch ⇒ st.show(t)
    case l: Leaf   ⇒ sl.show(l)
  }

}
