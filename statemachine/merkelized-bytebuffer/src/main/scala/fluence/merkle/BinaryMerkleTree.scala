package fluence.merkle

import java.nio.ByteBuffer
import java.util

import scala.collection.mutable
import scala.language.higherKinds

/**
 *
 * Balanced tree that connects with ByteBuffer through leaves.
 *
 * @param allNodes the full tree with hashes in one array
 * @param treeHeight the number of all rows in tree
 * @param mappedLeafCount the number of leaves that have data from storage
 * @param treeHasher the functionality to calculate hashes
 * @param memory the tree is based on this memory
 */
class BinaryMerkleTree private (
  val allNodes: Array[Array[Byte]],
  val treeHeight: Int,
  mappedLeafCount: Int,
  treeHasher: TreeHasher,
  memory: TrackingMemoryBuffer
) {
  import TreeMath._

  val leafsCount: Int = power2(treeHeight)
  private val chunkSize = memory.chunkSize
  private val startOfLeafIndex = leafsCount

  private def concatenate(l: Array[Byte], r: Array[Byte]): Array[Byte] =
    if (r == null) l
    else l ++ r

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

  def getHash: Array[Byte] = {
    calculateRootHash()
  }

  /**
   * Recalculates hashes of dirty chunks and ascend calculation to the root of the tree.
   *
   * @return root hash
   */
  def recalculateHash(): Array[Byte] = {
    val hash = recalculateLeafs(mappedLeafCount, memory.getDirtyChunks)
    memory.getDirtyChunks.clear()
    hash
  }

  // for test purpose only
  def recalculateAll(): Array[Byte] = {
    val allLeafs = mappedLeafCount
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
    if (pos < mappedLeafCount) {
      val index = startOfLeafIndex + pos - 1
      val offset = pos * chunkSize
      val bytes = memory.getChunk(offset)
      val newHash = treeHasher.digest(bytes)
      allNodes(index) = newHash
    }
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
        treeHasher.digest(concatenate(l, r))
      allNodes(getNodeIndex(height, pos)) = newHash
    }
  }

  private def calculateRootHash(): Array[Byte] = {
    allNodes(0) = treeHasher.digest(concatenate(allNodes(1), allNodes(2)))
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
    var dirtyNodeId = dirtyNodes.nextSetBit(0)
    while (dirtyNodeId >= 0 && dirtyNodeId < rowSize) {
      calculateNodeHash(height, dirtyNodeId)

      dirtyNodes.set(getParentPos(dirtyNodeId))

      dirtyNodeId = dirtyNodes.nextSetBit(dirtyNodeId + 1)
    }

    val nextHeight = height - 1
    if (nextHeight == 0) calculateRootHash()
    else {
      val parentsRowSize = (rowSize + 1) / 2
      recalculateNodes(parentsRowSize, nextHeight, dirtyNodes)
    }
  }

  def calculateNodeHashPrev(height: Int, pos: Int, value: Array[Byte]): Unit = {
    val (l, r) = getChildren(height, pos)
    val index = getNodeIndex(height, pos)
    if (l == r) {
      allNodes(index) = value
    } else {
      val newHash =
        treeHasher.digest(concatenate(l, r))
      allNodes(index) = newHash
    }
  }

  def fillLeafs(value: Array[Byte]): Unit = {
    val firstIndex = getNodeIndex(treeHeight, 0)
    for { i <- firstIndex until firstIndex + mappedLeafCount } yield {
      allNodes(i) = value
    }
  }

  def fillRow(height: Int, rowSize: Int, value: Array[Byte]): Unit = {
    for { i <- 0 until rowSize } yield {
      val index = getNodeIndex(height, 0)
      calculateNodeHashPrev(height, i, value)
    }
  }

  def initTree(): Unit = {
    val leafHash = treeHasher.digest(ByteBuffer.wrap(Array.fill(memory.chunkSize)(0)))
    fillLeafs(leafHash)
    val precalculatedHashes = mutable.Map.empty[Int, Array[Byte]]
    precalculatedHashes.put(treeHeight, leafHash)
    (1 until treeHeight).reverse.foldLeft[Int]((mappedLeafCount + 1) / 2) {
      case (rowSize, height) =>
        val previousHash = precalculatedHashes(height + 1)
        val hash = treeHasher.digest(previousHash ++ previousHash)
        fillRow(height, rowSize, hash)
        precalculatedHashes.put(height, hash)
        (rowSize + 1) / 2
    }
    calculateRootHash()

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

  def bytesToHex(hashInBytes: Array[Byte]): String = {
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

    val size = storage.capacity()
    if (size % chunkSize != 0)
      throw new RuntimeException(s"Size should be divided entirely on chunkSize. Size: $size, chunkSize: $chunkSize")

    // count of leaves that mapped on byte array (can have non-zero values)
    val mappedLeafCount = (size + chunkSize - 1) / chunkSize

    // number of leaves in ByteBuffer
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

    val tree =
      new BinaryMerkleTree(arrayTree, treeHeight, mappedLeafCount, treeHasher, storage)
    tree.initTree()
    tree
  }
}
