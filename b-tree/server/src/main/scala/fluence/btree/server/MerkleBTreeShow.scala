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
import fluence.btree.common.{ ClientPutDetails, Hash, ValueRef }
import fluence.btree.server.core.{ BTreePutDetails, NodeWithId }

/**
 * ''Show'' type class implementations for [[MerkleBTree]] entities.
 */
object MerkleBTreeShow {

  import fluence.btree.common.BTreeCommonShow._

  implicit def showBTreePutDetails(implicit cpd: Show[ClientPutDetails], svr: Show[ValueRef]): Show[BTreePutDetails] = {
    Show.show((bpd: BTreePutDetails) ⇒
      s"BTreePutDetails(${cpd.show(bpd.clientPutDetails)})"
    )
  }
  implicit def showBranch(implicit sani: Show[Array[NodeId]], sh: Show[Hash]): Show[Branch] = {
    Show.show((t: Branch) ⇒
      s"Branch(${t.keys}, ${sani.show(t.childsReferences)}, ${t.size}, ${sh.show(t.checksum)})"
    )
  }

  implicit def showLeaf(
    implicit
    savr: Show[Array[ValueRef]], sah: Show[Array[Hash]], sh: Show[Hash]
  ): Show[Leaf] = {
    Show.show((l: Leaf) ⇒
      s"""Leaf(${l.keys}, ${savr.show(l.valuesReferences)}, ${sah.show(l.valuesChecksums)},
         ${l.size}, ${sh.show(l.checksum)})"""
    )
  }

  implicit def showNodeWithId(implicit sn: Show[Node], sni: Show[NodeId]): Show[NodeWithId[NodeId, Node]] = {
    Show.show((n: NodeWithId[NodeId, Node]) ⇒ s"""NodeWithId(${sni.show(n.id)}, ${sn.show(n.node)})""")
  }

  implicit def showNode(implicit st: Show[Branch], sl: Show[Leaf]): Show[Node] = {
    case t: Branch @unchecked ⇒ st.show(t)
    case l: Leaf @unchecked   ⇒ sl.show(l)
  }

}
