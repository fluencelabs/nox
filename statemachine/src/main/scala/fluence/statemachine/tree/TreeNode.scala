/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.statemachine.tree

import fluence.statemachine.util.Crypto
import fluence.statemachine.{StoreKey, StoreValue}

import scala.collection.immutable.SortedMap

/**
 * Immutable key-value tree node.
 *
 * Keys are hierarchical, described by [[TreePath]], '/'-separated in string representation.
 *
 * No balancing performed, the node might contain arbitrary number of children.
 *
 * Modifying operations return a new tree that reuses unchanged branches of original tree. Updated branches of the
 * new tree are always not merkelized.
 *
 * @param children   child nodes
 * @param value      assigned value, if exists
 */
abstract class TreeNode(val children: Map[StoreKey, TreeNode], val value: Option[StoreValue]) {

  /**
   * Returns merklelized version of current node. It might be the same node if it already merkelized.
   *
   * @return new, merkelized, tree root
   */
  def merkelize(): MerkleTreeNode

  /**
   * Puts new value to a given key, without affecting children of the target node (the node whose value added/changed).
   * Creates the target and intermediate nodes if needed, values for newly created intermediate nodes are set to `None`.
   *
   * Resetting node's value to `None` back is not supported currently.
   *
   * @param key target key, described by the path relative from the current node
   * @param newValue new value
   * @return a node obtained from the current after the change
   */
  def putValue(key: TreePath[StoreKey], newValue: StoreValue): TreeNode = key match {
    case EmptyTreePath => SimpleTreeNode(children, Some(newValue))
    case SplittableTreePath(next, rest) =>
      SimpleTreeNode(
        children + (next -> children.getOrElse(next, TreeNode.emptyNode).putValue(rest, newValue)),
        this.value
      )
  }

  /**
   * Removes value for a given key.
   * Does not affect target node's children.
   * If the target node is not the root and becomes [[TreeNode.emptyNode]] (no children), it removed.
   *
   * @param key target key, described by the path relative from the current node
   * @return a node obtained from the current after the change
   */
  def removeValue(key: TreePath[StoreKey]): TreeNode = key match {
    case EmptyTreePath => SimpleTreeNode(children, None)
    case SplittableTreePath(next, rest) =>
      val nextChild = children.getOrElse(next, TreeNode.emptyNode).removeValue(rest)
      val newChildren = if (nextChild.isEmpty) children - next else children + (next -> nextChild)
      SimpleTreeNode(newChildren, this.value)
  }

  /**
   * Checks whether the [[TreeNode]] corresponding to the given key exists and contains some value.
   *
   * @param key target key, described by the path relative from the current node
   * @return whether the node exists and contains some value
   */
  def hasValue(key: TreePath[StoreKey]): Boolean = getValue(key).isDefined

  /**
   * Gets value of the [[TreeNode]] corresponding to the given key, if the node exists and the value is defined.
   *
   * @param key target key, described by the path relative from the current node
   * @return requested value, if exists and defined
   */
  def getValue(key: TreePath[StoreKey]): Option[StoreValue] = getNode(key).flatMap((x: TreeNode) => x.value)

  /**
   * Returns whether this node is empty which means that it has no children and no associated value.
   */
  def isEmpty: Boolean = value.isEmpty && children.isEmpty

  /**
   * TODO:
   *
   * @param keyTemplate
   * @return
   */
  def selectByTemplate(keyTemplate: TreePath[StoreKey]): Seq[TreePath[StoreKey]] = keyTemplate match {
    case EmptyTreePath => List(EmptyTreePath)
    case SplittableTreePath(next, rest) =>
      val matchedChildren: Seq[StoreKey] = next match {
        case "*" => children.keys.toList
        case key => List(key)
      }
      matchedChildren
        .flatMap(
          key => children.get(key).map(node => node.selectByTemplate(rest).map(path => SplittableTreePath(key, path)))
        )
        .flatten
  }

  /**
   * Dumps the node's subtree to a multiline string.
   *
   * @param name value used to describe the current node
   * @param level level used for indentation
   */
  def dump(name: String = "/", level: Int = 0): String = {
    val selfDump = " " * 2 * level + name + " -> " + value.getOrElse("∅")
    val childDump = children.map { case (key, child) => child.dump(key, level + 1) }
    (selfDump +: childDump.toSeq).mkString("\n")
  }

  /**
   * Gets the [[TreeNode]] corresponding to the given key, if exists.
   *
   * @param key target key, described by the path relative from the current node
   * @return requested node, if exists
   */
  protected def getNode(key: TreePath[StoreKey]): Option[TreeNode] = key match {
    case EmptyTreePath => Some(this)
    case SplittableTreePath(next, rest) => children.get(next).flatMap(_.getNode(rest))
  }
}

object TreeNode {

  /**
   * Empty node, has no value, no children, and not merkelized.
   */
  val emptyNode: TreeNode = SimpleTreeNode(SortedMap.empty[StoreKey, TreeNode], None)

  /**
   * Merkelized empty node, has no value, no children.
   */
  val emptyMerkelizedNode: MerkleTreeNode = SimpleTreeNode(SortedMap.empty[StoreKey, TreeNode], None).merkelize()
}

/**
 * Merklelized tree node. Merkle hash stored in the node.
 *
 * @param children   child nodes
 * @param value      assigned value, if exists
 * @param merkleHash hash of sub-tree rooted at this node
 */
case class MerkleTreeNode(
  override val children: Map[StoreKey, MerkleTreeNode],
  override val value: Option[StoreValue],
  merkleHash: MerkleHash
) extends TreeNode(children, value) {

  /**
   * Returns Merkle proof for value of [[TreeNode]] corresponding to the target key.
   *
   * @param key target key, described by the path relative from the current node
   */
  def getProof(key: TreePath[StoreKey]): MerkleProof = key match {
    case EmptyTreePath => MerkleProof.fromSingleLevel(merkleItems())
    case SplittableTreePath(next, rest) => children(next).getProof(rest).prepend(merkleItems())
  }

  override def merkelize(): MerkleTreeNode = this

  private def merkleItems(): List[MerkleHash] = MerkleTreeNode.merkleItems(children, value)
}

object MerkleTreeNode {

  def apply(children: Map[StoreKey, MerkleTreeNode], value: Option[StoreValue]): MerkleTreeNode =
    MerkleTreeNode(children, value, MerkleHash.merge(merkleItems(children, value), HexBasedDigestMergeRule))

  /**
   * Builds list of digests that used to produce node's hash.
   *
   * If value is defined it is: `H(value) || H(key_1) || H(child_1) || .. || H(key_N) || H(child_N)`.
   * Otherwise `H(value)` term is not prepended.
   *
   * Note that the odd number of merkle items corresponds to nodes with defined value,
   * whereas the even one corresponds to node with undefined value.
   *
   * Classical 2nd preimage attack strategy similar to
   * [[https://crypto.stackexchange.com/questions/2106/what-is-the-purpose-of-using-different-hash-functions-for-the-leaves-and-interna this]]
   * is not applied here because there are no distinction between branch and leaf nodes, the hash of leaf node is
   * `H(H(value))`.
   *
   * TODO: However, additional thorough security research is required here.
   *
   * @param children child nodes
   * @param value assigned value, if exists
   */
  def merkleItems(children: Map[StoreKey, MerkleTreeNode], value: Option[StoreValue]): List[MerkleHash] = {
    val childItems = children.flatMap(x => List(Crypto.sha3Digest256(x._1), x._2.merkleHash)).toList
    value.map(Crypto.sha3Digest256(_) :: childItems).getOrElse(childItems)
  }
}

/**
 * Not merkelized version of tree node. However, descendant nodes might be merkelized.
 *
 * @param children   child nodes
 * @param value      assigned value, if exists
 */
case class SimpleTreeNode(override val children: Map[StoreKey, TreeNode], override val value: Option[StoreValue])
    extends TreeNode(children, value) {
  override def merkelize(): MerkleTreeNode = MerkleTreeNode(children.mapValues(_.merkelize()), value)
}
