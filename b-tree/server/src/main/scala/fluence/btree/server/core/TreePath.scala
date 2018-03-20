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

package fluence.btree.server.core

import fluence.btree.server.core.TreePath.PathElem

/**
  * [[TreePath]] is the path traversed from the root to a leaf (leaf is excluded). Contains all the information you need
  * to climb up the tree.
  *
  * @param branches The path from root to leaf
  */
case class TreePath[Id, Node <: BranchNode[_, _]](branches: Seq[PathElem[Id, Node]]) {

  /**
    * Return parent of current node - in other words the previous node of the current node
    * (last element which was pushed into ''branches'').
    */
  def getParent: Option[PathElem[Id, Node]] =
    branches.lastOption

  /**
    * Pushes new tree element to tail of sequence. Returns bew version of [[TreePath]].
    * Doesn't change the original [[TreePath]]: returns a new [[TreePath]] instance instead.
    */
  def addBranch(newId: Id, newTree: Node, nextChildIdx: Int): TreePath[Id, Node] =
    copy(branches = branches :+ PathElem(newId, newTree, nextChildIdx))

}

object TreePath {

  def apply[Id, Node <: BranchNode[_, _]](nodeId: Id, node: Node, nextChildIdx: Int): TreePath[Id, Node] = {
    new TreePath(Seq(PathElem(nodeId, node, nextChildIdx)))
  }

  def empty[Id, Node <: BranchNode[_, _]]: TreePath[Id, Node] = new TreePath(Seq.empty)

  /**
    * Branch node with its corresponding id and next child position idx.
    *
    * @param branchId      Current branch node id (used for saving node to Store)
    * @param branch        Current branch node
    * @param nextChildIdx Next child position index.
    */
  case class PathElem[Id, Node <: BranchNode[_, _]](branchId: Id, branch: Node, nextChildIdx: Int)

}
