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

import cats.syntax.show._
import fluence.btree.common.BTreeCommonShow._
import fluence.btree.common.merkle.{ GeneralNodeProof, NodeProof }
import fluence.btree.common.{ Hash, Key, ValueRef }
import fluence.btree.server.MerkleBTreeShow._
import fluence.btree.server._
import fluence.crypto.hash.{ CryptoHasher, JdkCryptoHasher }

import scala.reflect.ClassTag

/**
 * Operations performed on nodes.
 *
 * @param cryptoHasher Hash service uses for calculating nodes checksums.
 */
private[server] class NodeOps(cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]]) {

  implicit class LeafOps(leaf: Leaf) extends LeafNode.Ops[Key, ValueRef] {

    override def rewrite(key: Key, valueRef: ValueRef, valueChecksum: Hash, idx: Int): Leaf = {
      assert(idx >= 0 && idx <= leaf.size, "Index should be between 0 and size of leaf")
      val keys = rewriteElementInArray(leaf.keys, key, idx)
      val vReferences = rewriteElementInArray(leaf.valuesReferences, valueRef, idx)
      val vChecksums = rewriteElementInArray(leaf.valuesChecksums, valueChecksum, idx)
      val kvChecksums = getKvChecksums(keys, vChecksums)
      val leafChecksum = getLeafChecksum(kvChecksums)

      LeafNode(keys, vReferences, vChecksums, kvChecksums, keys.length, leafChecksum)
    }

    override def insert(key: Key, valueRef: ValueRef, valueChecksum: Hash, idx: Int): Leaf = {
      assert(idx >= 0 && idx <= leaf.size, "Index should be between 0 and size of leaf")
      val keys = insertElementToArray(leaf.keys, key, idx)
      val vReferences = insertElementToArray(leaf.valuesReferences, valueRef, idx)
      val vChecksums = insertElementToArray(leaf.valuesChecksums, valueChecksum, idx)
      val kvChecksums = getKvChecksums(keys, vChecksums)
      val leafChecksum = getLeafChecksum(kvChecksums)

      LeafNode(keys, vReferences, vChecksums, kvChecksums, keys.length, leafChecksum)
    }

    override def split: (Leaf, Leaf) = {
      assert(leaf.size % 2 == 1, "Leaf size before splitting should be odd!")

      val splitIdx = leaf.size / 2
      val (leftKeys, rightKeys) = leaf.keys.splitAt(splitIdx)
      val (leftVRefs, rightVRefs) = leaf.valuesReferences.splitAt(splitIdx)
      val (leftVChecksums, rightVChecksums) = leaf.valuesChecksums.splitAt(splitIdx)

      val leftLeafKvChecksums = getKvChecksums(leftKeys, leftVChecksums)
      val rightLeafKvChecksums = getKvChecksums(rightKeys, rightVChecksums)

      val leftLeaf = LeafNode(
        leftKeys, leftVRefs, leftVChecksums, leftLeafKvChecksums, leftKeys.length, getLeafChecksum(leftLeafKvChecksums)
      )
      val rightLeaf = LeafNode(
        rightKeys, rightVRefs, rightVChecksums, rightLeafKvChecksums, rightKeys.length, getLeafChecksum(rightLeafKvChecksums)
      )

      leftLeaf → rightLeaf
    }

    override def toProof(substitutionIdx: Int): NodeProof = {
      GeneralNodeProof(Array.emptyByteArray, leaf.checksumsOfKv, substitutionIdx)
    }

  }

  implicit class BranchOps(branch: Branch) extends BranchNode.Ops[Key, NodeId] {

    def insertChild(key: Key, childRef: ChildRef[NodeId], insIdx: Int): BranchNode[Key, NodeId] = {

      val idx = if (isRightmost(branch) && insIdx == -1) {
        // this child for inserting is rightmost child of rightmost parent branch, we take last branch idx as insert index
        branch.size
      } else {
        assert(insIdx >= 0, s"Impossible to insert by negative index=$insIdx for regular branch=${branch.show}, key=$key")
        insIdx
      }

      val keys: Array[Key] = insertElementToArray(branch.keys, key, idx)
      val children: Array[NodeId] = insertElementToArray(branch.childsReferences, childRef.id, idx)
      val childsChecksums = insertElementToArray(branch.childsChecksums, childRef.checksum, idx)
      val nodeChecksum: Hash = getBranchChecksum(keys, childsChecksums)

      BranchNode(keys, children, childsChecksums, keys.length, nodeChecksum)
    }

    override def split: (Branch, Branch) = {
      val splitIdx = branch.size / 2
      val (leftKeys, rightKeys) = branch.keys.splitAt(splitIdx)
      val (leftChildren, rightChildren) = branch.childsReferences.splitAt(splitIdx)
      val (leftChildsChecksums, rightChildsChecksums) = branch.childsChecksums.splitAt(splitIdx)

      val leftBranch = BranchNode(
        leftKeys, leftChildren, leftChildsChecksums, leftKeys.length, getBranchChecksum(leftKeys, leftChildsChecksums)
      )
      val rightBranch = BranchNode(
        rightKeys, rightChildren, rightChildsChecksums, rightKeys.length, getBranchChecksum(rightKeys, rightChildsChecksums)
      )

      leftBranch → rightBranch
    }

    /** Returns ''true'' if current branch is rightmost (the last) on this level of tree, ''false'' otherwise. */
    private def isRightmost(branch: Branch): Boolean =
      branch.childsReferences.length > branch.size

    override def updateChildChecksum(newChildHash: Array[Byte], idx: Int): BranchNode[Key, NodeId] = {
      val newChildsChecksums = rewriteElementInArray(branch.childsChecksums, newChildHash, idx)
      branch.copy(
        childsChecksums = newChildsChecksums,
        checksum = getBranchChecksum(branch.keys, newChildsChecksums))
    }

    override def toProof(substitutionIdx: Int): NodeProof = {
      GeneralNodeProof(cryptoHasher.hash(branch.keys.flatten), branch.childsChecksums, substitutionIdx)
    }

  }

  /** Create new leaf with specified ''key'' and ''value''.*/
  def createLeaf(key: Key, valueRef: ValueRef, valueChecksum: Hash): Leaf = {
    val keys = Array(key)
    val vReferences = Array(valueRef)
    val vChecksums = Array(valueChecksum)
    val kvChecksums = getKvChecksums(keys, vChecksums)
    LeafNode(keys, vReferences, vChecksums, kvChecksums, 1, getLeafChecksum(kvChecksums))
  }

  /** Create new branch node with specified ''key'' and 2 child nodes. */
  def createBranch(key: Key, leftChild: ChildRef[NodeId], rightChild: ChildRef[NodeId]): Branch = {
    val keys = Array(key)
    val children = Array(leftChild.id, rightChild.id)
    val childsChecksums = Array(leftChild.checksum, rightChild.checksum)
    BranchNode(keys, children, childsChecksums, 1, getBranchChecksum(keys, childsChecksums))
  }

  /** Returns array of checksums for each key-value pair */
  private[server] def getKvChecksums(keys: Array[Key], values: Array[Hash]): Array[Hash] = {
    keys.zip(values).map { case (key, value) ⇒ cryptoHasher.hash(key, value) }
  }

  /** Returns checksum of leaf */
  def getLeafChecksum(hashedValues: Array[Hash]): Hash =
    GeneralNodeProof(Array.emptyByteArray, hashedValues, -1)
      .calcChecksum(cryptoHasher, None)

  /** Returns checksum of branch node */
  def getBranchChecksum(keys: Array[Key], childsChecksums: Array[Hash]): Hash =
    GeneralNodeProof(cryptoHasher.hash(keys.flatten), childsChecksums, -1)
      .calcChecksum(cryptoHasher, None)

  /**
   * Returns updated copy of array with the updated element for ''insIdx'' index.
   * We choose variant with array copying for prevent changing input parameters.
   * Work with mutable structures is more error-prone. It may be changed in the future by performance reason.
   */
  private def rewriteElementInArray[T : ClassTag](array: Array[T], insElem: T, insIdx: Int): Array[T] = {
    // todo perhaps, more optimal implementation might be needed with array mutation in the future
    val newArray = Array.ofDim[T](array.length)
    Array.copy(array, 0, newArray, 0, array.length)
    newArray(insIdx) = insElem
    newArray
  }

  /** Returns updated copy of array with the inserted element for ''insIdx'' index. */
  private def insertElementToArray[T : ClassTag](array: Array[T], insElem: T, insIdx: Int): Array[T] = {
    val newArray = Array.ofDim[T](array.length + 1)
    Array.copy(array, 0, newArray, 0, insIdx)
    Array.copy(array, insIdx, newArray, insIdx + 1, array.length - insIdx)
    newArray(insIdx) = insElem
    newArray
  }

}

private[server] object NodeOps {

  def apply(cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]] = JdkCryptoHasher.Sha256): NodeOps = {
    new NodeOps(cryptoHasher)
  }

}
