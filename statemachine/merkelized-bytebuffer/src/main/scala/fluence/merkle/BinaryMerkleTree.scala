package fluence.merkle

import java.util

import fluence.merkle.ops.{ByteMerkleOperations, MerkleOperations}
import fluence.merkle.storage.Storage

import scala.language.higherKinds
import scala.reflect.ClassTag

class BinaryMerkleTree private (
  val allNodes: Array[Array[Byte]],
  val treeHeight: Int,
  chunkSize: Int,
  mappedLeafCount: Int,
  ops: MerkleOperations[Array[Byte]],
  storage: Storage[Array[Byte]]
)(implicit m: ClassTag[Array[Byte]]) {
  import TreeMath._

  val leafsCount: Int = power2(treeHeight)
  private val startOfLeafIndex = leafsCount
  private val defaultLeafChunk = ops.defaultLeaf(chunkSize)

  /**
   * Calculates node index in array.
   *
   * @param height position of node in a 'column'
   * @param pos position of node in a row
   */
  private def getNodeIndex(height: Int, pos: Int) = {
    assert(height > 0 && height <= treeHeight)
    val startOfLine = power2(height)
    val index = startOfLine + pos - 1
    index
  }

  /**
   * Find children by node's position.
   *
   * @param height position of node in a 'column'
   * @param pos position of node in a row
   * @return
   */
  private def getChildren(height: Int, pos: Int): (Array[Byte], Array[Byte]) = {
    // leafs don't have children
    assert(height < treeHeight)
    val index = getNodeIndex(height + 1, pos * 2)
    if (index % 2 != 0) (allNodes(index), allNodes(index + 1))
    else (allNodes(index - 1), allNodes(index))
  }

  /**
   * Calculates parent position in a row that upper than child's row
   *
   * @param pos child position in a row
   */
  private def getParentPos(pos: Int) = pos / 2

  /**
   * Recalculates hashes of dirty chunks and ascend calculation to the root of the tree.
   *
   * @return root hash
   */
  def recalculateHash(): Array[Byte] = recalculateLeafs(leafsCount, storage.getDirtyChunks)

  // for test purpose only
  def recalculateAll(): Array[Byte] = {
    val allLeafs = leafsCount
    val bs = new util.BitSet(allLeafs)
    bs.set(0, allLeafs)
    recalculateLeafs(allLeafs, bs)
  }

  /**
   * Gets data from storage and calculates hash.
   *
   * @param pos leaf position
   */
  private def recalculateLeafHash(pos: Int): Unit = {
    val index = startOfLeafIndex + pos - 1
    val bytes = if (pos >= mappedLeafCount) {
      defaultLeafChunk
    } else {
      val offset = pos * chunkSize
      storage.getElements(offset, chunkSize)
    }

    val newHash = ops.hash(bytes)
    allNodes(index) = newHash
  }

  /**
   * Gets hashes from children, concatenate them and calculates hash.
   *
   * @param height row number in the tree
   * @param pos position in a row
   */
  private def calculateNodeHash(height: Int, pos: Int): Unit = {
    if (height == treeHeight) {
      recalculateLeafHash(pos)
    } else {
      val (l, r) = getChildren(height, pos)
      val newHash =
        ops.hash(ops.concatenate(l, r))
      allNodes(getNodeIndex(height, pos)) = newHash
    }
  }

  private def calculateRootHash(): Array[Byte] = {
    allNodes(0) = ops.hash(ops.concatenate(allNodes(1), allNodes(2)))
    allNodes(0)
  }

  /**
   * Recalculates all dirty nodes from a row of the tree.
   *
   * @param rowSize number of elements in a row
   * @param height number of a tree row
   * @param dirtyNodes list of dirty nodes
   *
   * @return root hash
   */
  @scala.annotation.tailrec
  private def recalculateNodes(rowSize: Int, height: Int, dirtyNodes: util.BitSet): Array[Byte] = {
    var i = -1
    var exit = true
    val parentsRowSize = rowSize / 2
    val parents = new util.BitSet(parentsRowSize)

    while (exit) {
      i = dirtyNodes.nextSetBit(i + 1)
      if (i < 0 || i == Integer.MAX_VALUE) exit = false
      else {
        calculateNodeHash(height, i)
        parents.set(getParentPos(i))
      }
    }

    val nextHeight = height - 1
    if (nextHeight == 0) calculateRootHash()
    else recalculateNodes(parentsRowSize, nextHeight, parents)
  }

  private def recalculateLeafs(size: Int, bits: util.BitSet): Array[Byte] = {
    recalculateNodes(size, treeHeight, bits)
  }

  def showTree(): Unit = {

    var treeNodesStr: Map[Int, List[String]] = Map.empty
    val spaceSize = 4

    val leafSize = (BinaryMerkleTree.bytesToHex(allNodes.last) + " " * spaceSize).length

    val bottomSize = leafSize * power2(treeHeight)

    treeNodesStr = treeNodesStr + (0 -> List(BinaryMerkleTree.bytesToHex(allNodes(0))))
    allNodes.zipWithIndex.drop(1).foreach {
      case (n, index) =>
        val height = log2(index + 1)
        treeNodesStr = treeNodesStr + (height -> treeNodesStr
          .get(height)
          .map(l => l :+ BinaryMerkleTree.bytesToHex(n))
          .getOrElse(List(BinaryMerkleTree.bytesToHex(n))))
    }

    val lineSize =
      treeNodesStr.map(_._2.foldLeft(0) { case (acc, str) => acc + str.length + spaceSize }).max
    treeNodesStr.map { case (k, v) => (k, v) }.toList.sortBy(_._1).foreach {
      case (_, els) =>
        els.zipWithIndex.foreach {
          case (str, i) =>
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

  def bytesToHex(hashInBytes: Array[Byte]): String = {
    val sb = new StringBuilder
    for (b <- hashInBytes) yield {
      sb.append(Integer.toHexString(b))
    }
    sb.toString
  }

  def apply(
    size: Int,
    chunkSize: Int,
    storage: Storage[Array[Byte]],
    hashFunc: Array[Byte] => Array[Byte]
  ): BinaryMerkleTree = {

    val operations = new ByteMerkleOperations(hashFunc)

    BinaryMerkleTree(size, chunkSize, operations, storage)
  }

  def apply(
    size: Int,
    chunkSize: Int,
    ops: MerkleOperations[Array[Byte]],
    storage: Storage[Array[Byte]]
  )(implicit c: ClassTag[Array[Byte]]): BinaryMerkleTree = {
    import TreeMath._

    assert(size % chunkSize == 0)

    // count of leafs that mapped on byte array (can have non-zero values)
    val mappedLeafCount = size / chunkSize + (if (size % chunkSize == 0) 0 else 1)

    // number of leafs in ByteBuffer
    val leafCount = {
      val power = log2(mappedLeafCount)
      val k = power2(power)
      if (k >= mappedLeafCount) k
      else power2(power + 1)
    }

    val treeHeight = {
      val h = log2(leafCount)
      if (h == 0) 1
      else h
    }

    val numberOfNodes = power2(treeHeight + 1) - 1

    val arrayTree = new Array[Array[Byte]](numberOfNodes)

    val tree = new BinaryMerkleTree(arrayTree, treeHeight, chunkSize, mappedLeafCount, ops, storage)
    tree.recalculateAll()
    tree
  }
}
