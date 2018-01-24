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
import fluence.btree.server.core.NodeWithId

/**
 * ''Show'' type class implementations for [[MerkleBTree]] entities.
 */
object MerkleBTreeShow {

  import fluence.btree.common.BTreeCommonShow._

  implicit def showLongArray: Show[Array[Long]] =
    Show.show((array: Array[Long]) ⇒ array.mkString("[", ",", "]"))

  implicit def showBranch(implicit sb: Show[Array[Array[Byte]]], sl: Show[Array[Long]]): Show[Branch] = {
    Show.show((t: Branch) ⇒ s"""Branch(${sb.show(t.keys)}, ${sl.show(t.childsReferences)}, ${t.size}, ${t.checksum.show})""")
  }

  implicit def showLeaf(implicit sb: Show[Array[Array[Byte]]], sl: Show[Array[Long]]): Show[Leaf] = {
    Show.show((l: Leaf) ⇒ s"""Leaf(${sb.show(l.keys)}, ${sl.show(l.valuesReferences)},
         ${sb.show(l.valuesChecksums)}, ${l.size}, ${l.checksum.show})""")
  }

  implicit def showNodeWithId(implicit sn: Show[Node]): Show[NodeWithId[NodeId, Node]] = {
    Show.show((n: NodeWithId[NodeId, Node]) ⇒ s"""NodeWithId(${n.id}, ${sn.show(n.node)})""")
  }

  implicit def showNode(implicit st: Show[Branch], sl: Show[Leaf]): Show[Node] = {
    case t: Branch @unchecked ⇒ st.show(t)
    case l: Leaf @unchecked   ⇒ sl.show(l)
  }

}
