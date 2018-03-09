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

import fluence.btree.common.Hash
import fluence.btree.common.merkle.NodeProof

/**
 * The root of tree elements hierarchy.
 */
sealed trait TreeNode[K] {

  /** Stored search keys */
  val keys: Array[K]

  /** Number of keys inside this tree node (optimization) */
  val size: Int

  /** Digest of this node state */
  val checksum: Hash
}

/**
 * A leaf element of tree, contains references of stored values with corresponded keys and other supporting data.
 * All leaves are located at the same depth (maximum depth) in the tree.
 * All arrays are in a one to one relationship. It means that:
 * {{{keys.size == checksumsOfValues.size == checksumsOfValues.size == checksumsOfKv.size == size }}}
 *
 * @param keys              Search keys
 * @param valuesReferences Stored values references
 * @param valuesChecksums  Array of checksums for each encrypted stored value (Not a checksum of value reference!)
 * @param kvChecksums      Array of checksums for each key and checksumsOfValue pair.
 *                           'hash(key+checksumsOfValue)' (optimization)
 * @param size              Number of keys inside this leaf. Actually size of each array in leaf. (optimization)
 * @param checksum         Hash of leaf state (hash of concatenated checksumsOfKv)
 * @param rightSibling     Reference to the right sibling leaf. Rightmost leaf don't have right sibling.
 *
 * @tparam K The type of search key
 * @tparam V The Type of stored value
 * @tparam C The type of reference to child nodes
 */
case class LeafNode[K, V, C](
    override val keys: Array[K],
    valuesReferences: Array[V],
    valuesChecksums: Array[Hash],
    kvChecksums: Array[Hash],

    override val size: Int,
    override val checksum: Hash,
    rightSibling: Option[C]
) extends TreeNode[K] {
  override def toString: String = LeafNode.show(this)
}

object LeafNode {

  /** Leaf operations. */
  trait Ops[K, V, C] {

    /**
     * Updates leaf with new data. Rewrites elements at the specified index position in the leaf.
     * Index should be between 0 and size of leaf. Doesn't change the original leaf: returns a new leaf instead.
     *
     * @param key            New Key for updating
     * @param valueRef       New Value Reference for updating
     * @param valueChecksum New Value Checksum for updating
     * @param idx            Index for rewriting
     * @return updated leaf
     */
    def rewrite(key: K, valueRef: V, valueChecksum: Hash, idx: Int): LeafNode[K, V, C]

    /**
     * Updates leaf with insert new data. Inserts elements at the specified index position in the leaf.
     * New elements will be placed at the specified index position, size of leaf will be increased by one.
     * Index should be between 0 and size of leaf. Doesn't change the original leaf: returns a new leaf instead.
     *
     * @param key             New key for inserting
     * @param valueRef       New Value Reference for updating
     * @param valueChecksum  New Value Checksum for updating
     * @param idx             Index for insertion
     * @return updated leaf
     */
    def insert(key: K, valueRef: V, valueChecksum: Hash, idx: Int): LeafNode[K, V, C]

    /**
     * Splits leaf into two approximately equal parts.
     * Doesn't change the original leaf: returns a two new leaf instead.
     *
     * @param rightLeafId Id for right leaf after splitting. Left leaf should takes old leaf id.
     *
     * @return tuple with 2 leafs
     */
    def split(rightLeafId: C): (LeafNode[K, V, C], LeafNode[K, V, C])

    /**
     * Returns node proof for current node
     *
     * @param substitutionIdx   Index for substitution childs checksum
     */
    def toProof(substitutionIdx: Int): NodeProof

  }

  def show(l: LeafNode[_, _, _]): String = {
    s"Leaf(keys=${l.keys.mkString(",")}, vRefs=${l.valuesReferences.mkString(",")}, " +
      s"vChecksums=${l.valuesChecksums.mkString(",")}, kvChecksums=${l.kvChecksums.mkString(",")}, " +
      s"size=${l.size}, checksum=${l.checksum}, rightSibling=${l.rightSibling})"
  }

}

/**
 * Branch node of tree, do not contains any business values, contains references to children nodes.
 * Number of children == number of keys in all branches except last(rightmost) for any tree level.
 * The rightmost branch contain (size + 1) children.
 *
 * @param keys              Search keys
 * @param childsReferences Children references
 * @param childsChecksums  Array of checksums for each child node (Not a checksum of child reference!) (optimization)
 * @param size              Number of keys inside this branch (optimization)
 * @param checksum         Hash of branch state
 *
 * @tparam K The type of searching Key
 * @tparam C The type of reference to child nodes
 */
case class BranchNode[K, C](
    override val keys: Array[K],
    childsReferences: Array[C],
    childsChecksums: Array[Hash],

    override val size: Int,
    override val checksum: Hash
) extends TreeNode[K] {
  override def toString: String = BranchNode.show(this)
}

object BranchNode {

  /** Branch node operations. */
  trait Ops[K, C] {

    /**
     * Inserts a child at the specified index position in the branch.
     * Doesn't change the original branch: returns a new branch instead.
     *
     * @param key       New key for inserting
     * @param childRef  New child node for inserting
     * @param idx       Index for insertion new child
     * @return Updated branch with inserted new key and child id
     */
    def insertChild(key: K, childRef: ChildRef[C], idx: Int): BranchNode[K, C]

    /**
     * Updates a checksum for the child by specified index and recalculates the branch checksum.
     * Doesn't change the original branch: returns a new branch instead.
     *
     * @param checksum New checksum of updated child
     * @param idx       Index for doing update
     * @return Updated branch node
     */
    def updateChildChecksum(checksum: Hash, idx: Int): BranchNode[K, C]

    /**
     * Updates checksum and reference in parent branch for the child by specified index;
     * recalculates the branch checksum. Doesn't change the original branch: returns a new branch instead.
     *
     * @param childRef New child reference (childs id and checksum)
     * @return Updated branch node
     */
    def updateChildRef(childRef: ChildRef[C], idx: Int): BranchNode[K, C]

    /**
     * Splits leaf into two approximately equal parts.
     *
     * @return tuple with 2 branch node
     */
    def split: (BranchNode[K, C], BranchNode[K, C])

    /**
     * Returns node proof for current node
     *
     * @param substitutionIdx   Index for substitution childs checksum
     */
    def toProof(substitutionIdx: Int): NodeProof

  }

  def show(b: BranchNode[_, _]): String = {
    s"Branch(keys=${b.keys.mkString(",")}, childsRefs=${b.childsReferences.mkString(",")}, " +
      s"size=${b.size}, checksum=${b.checksum})"
  }

}

/** Wrapper for the node with its corresponding id. */
case class NodeWithId[+Id, +Node <: TreeNode[_]](id: Id, node: Node)

/** Wrapper of the child id and the checksum. */
case class ChildRef[Id](id: Id, checksum: Hash)
