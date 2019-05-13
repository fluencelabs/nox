package fluence.merkle

import java.util

import scala.language.higherKinds

class BinaryMerkleTree private (
  val allNodes: Array[Array[Byte]],
  val treeHeight: Int,
  chunkSize: Int,
  mappedLeafCount: Int,
  hashFunc: Array[Byte] => Array[Byte],
  storage: TrackingMemoryBuffer
) {
  import TreeMath._

  val leafsCount: Int = power2(treeHeight)
  private val startOfLeafIndex = leafsCount
  private val defaultLeafChunk = defaultLeaf(chunkSize)

  private def hash(t: Array[Byte]): Array[Byte] = hashFunc(t)

  private def concatenate(l: Array[Byte], r: Array[Byte]): Array[Byte] = l ++ r

  private def defaultLeaf(chunkSize: Int): Array[Byte] = Array.fill(chunkSize)(0)

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

    val newHash = hash(bytes)
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
        hash(concatenate(l, r))
      allNodes(getNodeIndex(height, pos)) = newHash
    }
  }

  private def calculateRootHash(): Array[Byte] = {
    allNodes(0) = hash(concatenate(allNodes(1), allNodes(2)))
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
    val parentsRowSize = rowSize / 2
    val parents = new util.BitSet(parentsRowSize)

    var dirtyNodeId = dirtyNodes.nextSetBit(0)
    while (dirtyNodeId >= 0 && dirtyNodeId != Integer.MAX_VALUE) {
      calculateNodeHash(height, dirtyNodeId)
      parents.set(getParentPos(dirtyNodeId))
      dirtyNodeId = dirtyNodes.nextSetBit(dirtyNodeId + 1)
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
    hashFunc: Array[Byte] => Array[Byte],
    storage: TrackingMemoryBuffer
  ): BinaryMerkleTree = {
    import TreeMath._

    assert(size % chunkSize == 0)

    // count of leafs that mapped on byte array (can have non-zero values)
    val mappedLeafCount = (size + chunkSize - 1) / chunkSize

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

    val tree = new BinaryMerkleTree(arrayTree, treeHeight, chunkSize, mappedLeafCount, hashFunc, storage)
    tree.recalculateAll()
    tree
  }
}
