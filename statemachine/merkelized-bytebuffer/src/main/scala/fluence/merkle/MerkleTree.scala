package fluence.merkle

import java.util

import org.apache.commons.lang3.StringUtils

import scala.language.higherKinds
import scala.reflect.ClassTag

class MerkleTree[Storage[_], HashType, ElementType] private (
  val nodes: Array[HashType],
  val treeHeight: Int,
  chunkSize: Int
)(implicit M: MerkleOperations[Storage, ElementType, HashType], m: ClassTag[HashType]) {
  import TreeMath._

  val leafsCount = power2(treeHeight)
  val startOfLeafIndex = leafsCount

  /**
   * Calculates node index in array.
   *
   * @param height position of node in a 'column'
   * @param pos position of node in a row
   */
  private def getNodeIndex(height: Int, pos: Int) = {
    if (height < 0 || height > treeHeight) throw new RuntimeException("invalid height")
    val startOfLine = power2(height)
    val index = startOfLine + pos - 1
    index
  }

  /**
   * Find childrens by node's position.
   *
   * @param height position of node in a 'column'
   * @param pos position of node in a row
   * @return
   */
  private def getChildrens(height: Int, pos: Int): Array[HashType] = {
    if (height == treeHeight) throw new RuntimeException("No childrens on leafs")
    getBatch(height + 1, pos * 2)
  }

  /**
   * Gets node with its neighbor
   */
  private def getBatch(height: Int, pos: Int): Array[HashType] = {
    val index = getNodeIndex(height, pos)
    if (index == 0) Array(nodes(0))
    else if (index % 2 != 0) nodes.slice(index, index + 2)
    else nodes.slice(index - 1, index + 1)
  }

  /**
   * Calculates parent position in a row that upper than child's row
   *
   * @param pos child position in a row
   */
  private def getParentPos(pos: Int) = pos / 2

  /**
   * Recalculates hashes of touched chunks and touched ascending chain of nodes.
   *
   * @return
   */
  def recalculateHash(): HashType = recalculateLeafs(leafsCount, M.getAffectedChunks)

  def recalculateAll(): HashType = recalculateHash((0 until nodes.length / 2).toArray, treeHeight)

  private def rehashLeaf(chunkIndex: Int): Unit = {
    val index = startOfLeafIndex + chunkIndex - 1
    val offset = chunkIndex * chunkSize
    val bytes = M.get(offset, chunkSize)
    val newHash = M.hash(bytes)
    nodes(index) = newHash
  }

  private def rehashNode(height: Int, pos: Int): Unit = {
    if (height == treeHeight) {
      rehashLeaf(pos)
    } else {
      val childrens = getChildrens(height, pos).map(v => M.hashH(v)).toList
      val newHash =
        M.hashH(M.append(childrens))
      nodes(getNodeIndex(height, pos)) = newHash
    }
  }

  private def rehashRoot() = {
    val childrens = nodes.slice(1, 3).toList
    nodes(0) = M.hashH(M.append(childrens))
    nodes(0)
  }

  @scala.annotation.tailrec
  private def recalculateNodes(size: Int, height: Int, bits: util.BitSet): HashType = {
    var i = -1
    var exit = true
    val parentsSize = size / 2
    val parents = new util.BitSet(parentsSize)
    while (exit) {
      i = bits.nextSetBit(i + 1)
      if (i < 0 || i == Integer.MAX_VALUE) exit = false
      else {
        rehashNode(height, i)
        parents.set(getParentPos(i))
      }
    }
    val nextHeight = height - 1
    if (height == 0) rehashRoot()
    else recalculateNodes(parentsSize, height - 1, parents)
  }

  private def recalculateLeafs(size: Int, bits: util.BitSet): HashType = {
    recalculateNodes(size, treeHeight, bits)
  }

  @scala.annotation.tailrec
  final private def recalculateHash(affectedChunks: Array[Int], height: Int): HashType = {
    if (height == 0) {
      val childrens = getChildrens(0, 0).map(el => M.hashH(el)).toList
      nodes(0) = M.hashH(M.append(childrens))
      nodes(0)
    } else {
      val affectedParents: Array[Int] = for (i <- affectedChunks) yield {
        val index = getNodeIndex(height, i)
        if (height == treeHeight) {
          rehashLeaf(i)
        } else {
          rehashNode(height, i)
        }
        getParentPos(i)
      }
      recalculateHash(
        affectedParents,
        height - 1
      )
    }
  }

  def showTree(): Unit = {

    var treeNodesStr: Map[Int, List[String]] = Map.empty
    val spaceSize = 4

    val leafSize = (nodes.last.toString + " " * spaceSize).length

    val bottomSize = leafSize * power2(treeHeight)

    treeNodesStr = treeNodesStr + (0 -> List(nodes(0).toString))
    nodes.zipWithIndex.drop(1).foreach {
      case (n, index) =>
        val height = log2(index + 1)
        treeNodesStr = treeNodesStr + (height -> treeNodesStr
          .get(height)
          .map(l => l :+ n.toString)
          .getOrElse(List(n.toString)))
    }

    println(treeNodesStr.keys.mkString(", "))

    val lineSize =
      treeNodesStr.map(_._2.foldLeft(0) { case (acc, str) => acc + str.length + spaceSize }).max
    treeNodesStr.map { case (k, v) => (k, v) }.toList.sortBy(_._1).foreach {
      case (_, els) =>
        els.zipWithIndex.foreach {
          case (s, i) =>
            val si = scala.math.ceil(lineSize.toDouble / els.size).toInt
            print(StringUtils.center(s, si))
        }
        println()
    }
  }
}

trait Storage[Element] {
  def getElements(offset: Int, length: Int): Element
  def getAffectedChunks: util.BitSet
}

trait Hash[Raw, Hash] {
  def hash(value: Raw): Hash
}

trait MerkleOperations[S[_], ElementType, HashType] {
  def default: ElementType
  def get(offset: Int, length: Int): ElementType
  def hash(t: ElementType): HashType
  def hashH(t: HashType): HashType
  def getAffectedChunks: util.BitSet
  def append(l: List[HashType]): HashType
}

case class StringBufferStorage(size: Int, chunkSize: Int) extends Storage[String] {
  val sb = new StringBuffer("0" * size)

  var affectedChunks = new java.util.BitSet(size)

  override def getElements(offset: Int, length: Int): String = {
    val sb1 = new StringBuffer("0" * length)
    // return zeros if offset is greater then length
    val res =
      if (sb.length() <= offset) sb1.toString
      else {
        if (sb.length() >= offset + length) sb.substring(offset, offset + length)
        else sb.substring(offset, sb.length()) + sb1.substring(0, sb.length() - offset)
      }
    res
  }

  def getAffectedChunks: util.BitSet = {
    affectedChunks
  }

  def put(v: String, start: Int, end: Int): Unit = {
    sb.replace(start, end, v)
    println("start: " + start)
    println("end: " + end)
    affectedChunks.set(start / chunkSize, end / chunkSize)
  }
}

class StringBufferOperations(implicit S: Storage[String]) extends MerkleOperations[Storage, String, String] {
  override def default: String = "0"

  override def get(offset: Int, length: Int): String = S.getElements(offset, length)

  override def getAffectedChunks: util.BitSet = S.getAffectedChunks

  override def hash(t: String): String = t

  override def append(l: List[String]): String = l.mkString("")

  override def hashH(t: String): String = t
}

object MerkleTree {

  /*def apply(size: Int, chunkSize: Int): (ByteBufferWrapper, MerkleTree[ByteBufferWrapper, Array[Byte]]) = {

    val init: Int => ByteBufferWrapper = { bbSize =>
      val array = Array.fill[Byte](bbSize)(0)
      new ByteBufferWrapper(ByteBuffer.wrap(array), chunkSize)
    }

    val getBytes: (ByteBufferWrapper, Int, Int) => Array[Byte] = (bb: ByteBufferWrapper, offset: Int, length: Int) => {
      val array = new Array[Byte](length)
      bb.position(offset)
      val remaining = bb.remaining()
      val lengthToGet = if (remaining < length) remaining else length
      bb.get(array, 0, lengthToGet)
      array
    }

    val getAffectedChunks: ByteBufferWrapper => Set[Int] = (bb: ByteBufferWrapper) => {
      bb.getTouchedAndReset()
    }

    implicit val showDep: Show[Array[Byte]] = new Show[Array[Byte]] {
      override def show(t: Array[Byte]): String = t.mkString("")
    }

    MerkleTree[ByteBufferWrapper, Array[Byte]](size, chunkSize, init, Array.fill(1)(0), getBytes, getAffectedChunks)
  }*/

  def apply[S[_], A, H](
    size: Int,
    chunkSize: Int
  )(implicit M: MerkleOperations[S, A, H], c: ClassTag[H]): MerkleTree[S, H, A] = {
    import TreeMath._
    val childrenCount = 2

    // size of one bulk
    val childrenBulkSize = childrenCount * chunkSize

    // number of bulks in ByteBuffer
    //  val childrenBulkCount = size / childrenBulkSize + (if (size % childrenBulkSize == 0) 0 else 1)

    // count of leafs that mapped on byte array (can have non-zero values)
    val mappedLeafCount = size / chunkSize + (if (size % chunkSize == 0) 0 else 1)

    // number of leafs in ByteBuffer
    val leafCount = {
      val power = log2(mappedLeafCount)
      val k = power2(power)
      if (k >= mappedLeafCount) k
      else power2(power + 1)
    }

    println("mapped leaf count: " + mappedLeafCount)
    println("leaf count: " + leafCount)

    val treeHeight = {
      val h = log2(leafCount)
      if (h == 0) 1
      else h
    }

    val numberOfNodes = power2(treeHeight + 1) - 1

    val arrayTree = new Array[H](numberOfNodes)
    for (i <- 0 until numberOfNodes) {
      if (i < mappedLeafCount) {
        arrayTree(i) = M.hash(M.get(i * chunkSize, chunkSize))
      } else {
        arrayTree(i) = M.hash(M.default)
      }
    }
    new MerkleTree(arrayTree, treeHeight, chunkSize)
  }
}
