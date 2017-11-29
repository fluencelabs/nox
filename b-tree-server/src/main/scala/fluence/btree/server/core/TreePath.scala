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
