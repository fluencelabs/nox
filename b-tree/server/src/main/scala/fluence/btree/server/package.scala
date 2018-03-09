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

package fluence.btree

import fluence.btree.common.{ Key, ValueRef }
import fluence.btree.server.core.{ BranchNode, LeafNode, NodeWithId, _ }
import monix.eval.Task

package object server {

  type NodeId = Long

  type Node = TreeNode[Key]
  type Branch = BranchNode[Key, NodeId]
  type Leaf = LeafNode[Key, ValueRef, NodeId]

  type NodeAndId = NodeWithId[NodeId, Node]
  type Trail = TreePath[NodeId, Branch]

  type Get = GetCommand[Task, Key, ValueRef, NodeId]
  type Put = PutCommand[Task, Key, ValueRef, NodeId]

}
