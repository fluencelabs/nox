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

package fluence.merkle

import java.nio.ByteBuffer
import java.util

import scala.language.higherKinds

/**
 *
 * Balanced tree that connects with ByteBuffer through leaves. Not thread safe.
 *
 *
 * @param nodes the full tree with hashes in one array
 * @param treeHeight the number of all rows in tree
 * @param nonZeroLeafsCount the number of leaves that have data from storage
 * @param treeHasher the functionality to calculate hashes
 * @param memory the tree is based on this memory
 */
class BinaryMerkleTree private (
  val nodes: Array[Array[Byte]],
  val treeHeight: Int,
  nonZeroLeafsCount: Int,
  firstLeafId: Int,
  treeHasher: TreeHasher,
  memory: TrackingMemoryBuffer
) {
  import TreeMath._

  private val chunkSize = memory.chunkSize

  private def concatenate(l: Array[Byte], r: Array[Byte]): Array[Byte] =
    if (r == null) l
    else l ++ r

  /**
   * Calculates node index in array.
   *
   * @param treeLevel a tree level where node is located (counted from the counted from the root to bottom)
   * @param pos a position of node in this level
   */
  private def getNodeIndex(treeLevel: Int, pos: Int) = {
    assert(treeLevel > 0 && treeLevel <= treeHeight)
    val startOfLine = power2(treeLevel)
    val index = startOfLine + pos - 1
    index
  }

  /**
   * Find children by node's position.
   *
   * @param treeLevel a tree level where node is located (counted from the counted from the root to bottom)
   * @param pos a position of node in this level
   */
  private def getChildren(treeLevel: Int, pos: Int): (Array[Byte], Array[Byte]) = {
    // leafs don't have children
    assert(treeLevel < treeHeight)
    val index = getNodeIndex(treeLevel + 1, pos * 2)

    (nodes(index), nodes(index + 1))
  }

  /**
   * Calculates parent position in a level that upper than child's row
   *
   * @param pos child position in a level
   */
  private def getParentPos(pos: Int) = pos / 2

  /**
   * Returns current calculated hash without any state changes.
   */
  def getHash: Array[Byte] = nodes(0)

  /**
   * Recalculates hashes of dirty chunks and ascend calculations to the root of the tree.
   * After this list of dirty chunks will be cleared.
   *
   * @return root hash
   */
  def recalculateHash(): Array[Byte] = {
    val hash = recalculateLeafs(nonZeroLeafsCount, memory.getDirtyChunks)
    memory.clearDirtyChunks()
    hash
  }

  /**
   * Gets data from storage and calculates hash.
   *
   * @param pos leaf position
   */
  private def recalculateLeafHash(pos: Int): Unit = {
    if (pos < nonZeroLeafsCount) {
      val offset = pos * chunkSize
      val bytes = memory.getChunk(offset)
      val newHash = treeHasher.digest(bytes)

      val index = firstLeafId + pos - 1
      nodes(index) = newHash
    }
  }

  /**
   * Gets hashes from children, concatenate them and calculates hash.
   *
   * @param treeLevel a tree level where node is located (counted from the counted from the root to bottom)
   * @param pos a position of node in this level
   */
  private def calculateNodeHash(treeLevel: Int, pos: Int): Unit = {
    if (treeLevel == treeHeight) {
      recalculateLeafHash(pos)
    } else {
      val (l, r) = getChildren(treeLevel, pos)
      val newHash =
        treeHasher.digest(concatenate(l, r))
      nodes(getNodeIndex(treeLevel, pos)) = newHash
    }
  }

  private def calculateRootHash(): Array[Byte] = {
    nodes(0) = treeHasher.digest(concatenate(nodes(1), nodes(2)))
    nodes(0)
  }

  /**
   * Recalculates all dirty nodes from a row of the tree.
   *
   * @param levelSize number of elements in a level
   * @param treeLevel a tree level where node is located (counted from the counted from the root to bottom)
   * @param dirtyNodes list of dirty nodes
   *
   * @return root hash
   */
  @scala.annotation.tailrec
  private def recalculateNodes(levelSize: Int, treeLevel: Int, dirtyNodes: util.BitSet): Array[Byte] = {
    var dirtyNodeId = dirtyNodes.nextSetBit(0)
    while (dirtyNodeId >= 0 && dirtyNodeId < levelSize) {
      calculateNodeHash(treeLevel, dirtyNodeId)

      dirtyNodes.set(dirtyNodeId, false)
      dirtyNodes.set(getParentPos(dirtyNodeId))

      dirtyNodeId = dirtyNodes.nextSetBit(dirtyNodeId + 1)
    }

    val nextLevel = treeLevel - 1
    if (nextLevel == 0) calculateRootHash()
    else {
      val parentsRowSize = (levelSize + 1) / 2
      recalculateNodes(parentsRowSize, nextLevel, dirtyNodes)
    }
  }

  /**
   * Fills the tree with all hashes for empty memory.
   */
  private def initTree(): Unit = {

    // fills all leaves with hashes of empty byte array with the size of chunkSize
    def fillLeaves(value: Array[Byte]): Unit = {
      val firstIndex = getNodeIndex(treeHeight, 0)
      for { i <- firstIndex until firstIndex + nonZeroLeafsCount } yield {
        nodes(i) = value
      }
    }

    // fills one row with precalculated hash
    def fillRow(treeLevel: Int, rowSize: Int, value: Array[Byte]): Unit = {
      for { i <- 0 until (rowSize - 1) } yield {
        val index = getNodeIndex(treeLevel, i)
        nodes(index) = value
      }

      // last node in a row could have empty right child or different children, check this and calculate new hash if needed
      val index = getNodeIndex(treeLevel, rowSize - 1)
      val (l, r) = getChildren(treeLevel, rowSize - 1)
      if (l != r) {
        val newHash =
          treeHasher.digest(concatenate(l, r))
        nodes(index) = newHash
      } else nodes(index) = value

    }

    val leafHash = treeHasher.digest(ByteBuffer.wrap(Array.fill(memory.chunkSize)(0)))
    fillLeaves(leafHash)

    // go through all rows except row with leaves
    // new levelSize calculation depends on previous levelSize
    // new hash calculation depends on previous hash
    (1 until treeHeight).reverse.foldLeft[(Int, Array[Byte])](((nonZeroLeafsCount + 1) / 2, leafHash)) {
      case ((rowSize, previousHash), treeLevel) =>
        val hash = treeHasher.digest(previousHash ++ previousHash)
        fillRow(treeLevel, rowSize, hash)
        ((rowSize + 1) / 2, hash)
    }
    calculateRootHash()
  }

  private def recalculateLeafs(size: Int, bits: util.BitSet): Array[Byte] = {
    recalculateNodes(size, treeHeight, bits)
  }

  // shows the tree in `sout`. For test purpose only.
  private[this] def showTree(): Unit = {

    var treeNodesStr: Map[Int, List[String]] = Map.empty
    val spaceSize = 4

    val leafSize = (BinaryMerkleTree.bytesToHex(nodes.last) + " " * spaceSize).length

    val bottomSize = leafSize * power2(treeHeight)

    treeNodesStr = treeNodesStr + (0 -> List(BinaryMerkleTree.bytesToHex(nodes(0))))
    nodes.zipWithIndex.drop(1).foreach {
      case (n, index) =>
        val treeLevel = log2(index + 1)
        treeNodesStr = treeNodesStr + (treeLevel -> treeNodesStr
          .get(treeLevel)
          .map(l => l :+ BinaryMerkleTree.bytesToHex(n))
          .getOrElse(List(BinaryMerkleTree.bytesToHex(n))))
    }

    val lineSize =
      treeNodesStr.map(_._2.foldLeft(0) { case (acc, str) => acc + str.length + spaceSize }).max
    treeNodesStr.map { case (k, v) => (k, v) }.toList.sortBy(_._1).foreach {
      case (_, els) =>
        els.foreach { str =>
          val size = scala.math.ceil(lineSize.toDouble / els.size).toInt
          val strLen = str.length
          val pads = " " * ((size - strLen) / 2)

          print(pads + str + pads)
        }
        println()
    }
  }
}

object BinaryMerkleTree {

  private def bytesToHex(hashInBytes: Array[Byte]): String = {
    val sb = new StringBuilder
    if (hashInBytes == null) {
      sb.append("E")
    } else {
      for (b <- hashInBytes) yield {
        sb.append(Integer.toHexString(b))
      }
    }
    sb.toString
  }

  /**
   * Constructor to allocation nodes and leaves in the tree, calculation basic constants and hash of empty tree.
   * Capacity of storage should be dividable by `chunkSize`
   *
   * @param treeHasher the functionality to calculate hashes
   * @param storage the structure that tree will be based on
   * @throws RuntimeException if capacity of storage is not dividable by `chunkSize`
   * @return tree with calculated hash
   */
  def apply(
    treeHasher: TreeHasher,
    storage: TrackingMemoryBuffer
  ): BinaryMerkleTree = {
    import TreeMath._

    val chunkSize = storage.chunkSize

    val storageSize = storage.capacity()
    if (storageSize % chunkSize != 0)
      throw new RuntimeException(
        s"Size should be divided entirely on chunkSize. Size: $storageSize, chunkSize: $chunkSize"
      )

    // count of leaves that mapped on byte array (can have non-zero values)
    val nonZeroLeafsCount = (storageSize + chunkSize - 1) / chunkSize

    // number of leaves in ByteBuffer
    val leafCount = {
      val power = log2(nonZeroLeafsCount)
      val k = power2(power)
      if (k >= nonZeroLeafsCount) k
      else power2(power + 1)
    }

    val treeLevel = {
      val h = log2(leafCount)
      if (h == 0) 1
      else h
    }

    val numberOfNodes = power2(treeLevel + 1) - 1

    val arrayTree = new Array[Array[Byte]](numberOfNodes)

    val tree =
      new BinaryMerkleTree(arrayTree, treeLevel, nonZeroLeafsCount, leafCount, treeHasher, storage)
    tree.initTree()
    tree
  }
}
