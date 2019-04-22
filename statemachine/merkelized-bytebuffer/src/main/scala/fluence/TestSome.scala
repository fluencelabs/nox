package fluence
import java.nio.ByteBuffer

import cats.Show
import scodec.bits.ByteVector

case class Hash[T: Show](hash: T) {
  override def toString: String = Show[T].show(hash)
}

object TreeMath {

  def log2(input: Int): Int = {
    val res = 31 - Integer.numberOfLeadingZeros(input)
    res
  }

  def log2Ceiling(input: Int): Int = {
    val log2I = log2(input)
    val log2Next = log2(input + 1)
    if (log2Next > log2I) log2Next else log2I + 1
  }

  def power2(power: Int) = {
    1 << power
  }
}

object MerkleTreeBuilder {

  def apply[A](size: Int, chunkSize: Int): MerkleTreeBuilder[A] = {
    new MerkleTreeBuilder(size, chunkSize)
  }
}

class MerkleTreeBuilder[A](size: Int, chunkSize: Int) {
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

  val treeHeight = log2(leafCount)

  val numberOfNodes = power2(treeHeight + 1) - 1

  def fillTree[I, H: Show: Append](
    init: Int => I,
    hashFunc: H => H,
    default: H,
    get: (I, Int, Int) => H
  ): (I, MerkleTree[I, H]) = {
    val struct = init(size)
    val arrayTree = new Array[Hash[H]](numberOfNodes)
    for (i <- 0 until numberOfNodes) {
      if (i < mappedLeafCount) {
        arrayTree(i) = Hash(hashFunc(get(struct, i * chunkSize, chunkSize)))
      } else {
        arrayTree(i) = Hash(hashFunc(default))
      }

    }
    (struct, MerkleTree(arrayTree, treeHeight, struct, get, chunkSize, hashFunc))
  }
}

case class MerkleTree[I, H: Show: Append](
  nodes: Array[Hash[H]],
  treeHeight: Int,
  struct: I,
  get: (I, Int, Int) => H,
  chunkSize: Int,
  hashFunc: H => H
) {
  import TreeMath._

  private def getNodeIndex(height: Int, pos: Int) = {
    if (height < 0 || height > treeHeight) throw new RuntimeException("invalid height")
    val startOfLine = power2(height)
    println(s"getNodeIndex: height: $height, pos: $pos")
    println("start of line: " + startOfLine)
    val index = startOfLine + pos - 1
    println("index: " + index)
    index
  }

  private def getRoot: Hash[H] = {
    nodes(0)
  }

  private def getChildrens(height: Int, pos: Int): Array[Hash[H]] = {
    if (height == treeHeight) throw new RuntimeException("No childrens on leafs")
    getBatch(height + 1, pos * 2)
  }

  /**
   * Gets node with its neighbor
   */
  private def getBatch(height: Int, pos: Int): Array[Hash[H]] = {
    val index = getNodeIndex(height, pos)
    if (index == 0) Array(nodes(0))
    else if (index % 2 != 0) nodes.slice(index, index + 2)
    else nodes.slice(index - 1, index + 1)
  }

  private def getParentPos(pos: Int) = {
    pos / 2
  }

  @scala.annotation.tailrec
  final def recalculateHash(affectedChunks: Set[Int], height: Int = treeHeight): H = {
    if (height == 0) {
      println(s"height: $height")
      val childrens = getChildrens(0, 0).map(_.hash).toList
      nodes(0) = Hash(hashFunc(Append[H].append(childrens)))
      nodes(0).hash
    } else {
      val affectedParents: Set[Int] = for (i <- affectedChunks) yield {
        val index = getNodeIndex(height, i)
        if (height == treeHeight) {
          val offset = i * chunkSize
          val length = chunkSize
          println(s"offset: $offset, length: $length")
          println(s"height: $height, i: $i")
          println(s"index: $index")
          val bytes = get(struct, offset, length)
          println("BYTES === " + bytes)
          val newHash = Hash(hashFunc(bytes))
          println("NEW LEAF HASH: " + newHash)
          nodes(index) = newHash
        } else {
          println(s"height: $height, i: $i, index: $index")
          val childrens = getChildrens(height, i).map(_.hash).toList
          println(s"childrens: ${childrens.mkString(", ")}")
          val newHash =
            Hash(hashFunc(Append[H].append(childrens)))
          println("NEW HASH: " + newHash)
          nodes(index) = newHash
        }
        getParentPos(i)
      }
      println("affected parents: " + affectedParents)
      recalculateHash(
        affectedParents,
        height - 1
      )
    }

  }

  def showTree(): Unit = {
    print(nodes(0).toString)
    println()
    nodes.zipWithIndex.drop(1).foldLeft(2) {
      case (goingAheadHeight, (hash, index)) =>
        val nextHeight = log2(index + 2)
        print(hash.toString)
        if (nextHeight < goingAheadHeight) {
          print(" || ")
          goingAheadHeight
        } else {
          println()
          goingAheadHeight + 1
        }
    }
    println()
  }
}

object TestSome extends App {

  // ByteBuffer size
  val size = 13

  // size of one chunk to hash
  val chunkSize = 2

  val treeBuilder = new MerkleTreeBuilder(size, chunkSize)

  println("leaf count: " + treeBuilder.leafCount)
  println("mapped number: " + treeBuilder.mappedLeafCount)

  println("tree height: " + treeBuilder.treeHeight)
  println("tree number of nodes: " + treeBuilder.numberOfNodes)

  implicit val showDep: Show[String] = Show.fromToString

  val init: Int => ByteBuffer = { bbSize =>
    val array = Array.fill[Byte](bbSize)(0)
    println("INIT! size: " + bbSize)
    ByteBuffer.wrap(array)
  }

  val getBytes: (ByteBuffer, Int, Int) => String = (bb: ByteBuffer, offset: Int, length: Int) => {
    val array = new Array[Byte](length)
    bb.position(offset)
    val remaining = bb.remaining()
    val lengthToGet = if (remaining < length) remaining else length
    bb.get(array, 0, lengthToGet)
    ByteVector(array).toHex
  }

  val (bb, tree) = treeBuilder.fillTree[ByteBuffer, String](init, str => {
    "enc[" + str + "]"
  }, "0000", getBytes)
  println(tree.nodes.mkString(", "))

  println()
  println("====================================================================")
  println()
  tree.showTree()
  println()
  println("====================================================================")
  println()

  val n1 = 0
  val n2 = 1
  val n3 = 5
  bb.put(n1 * chunkSize, 1)
  bb.put(n2 * chunkSize, 7)
  bb.put(n3 * chunkSize, 127)
  tree.recalculateHash(Set(n1, n2, n3))
  tree.showTree()

  val n4 = 0
  bb.put(n4 * chunkSize + 1, 1)
  tree.recalculateHash(Set(n4))
  tree.showTree()

  val n5 = 1
  bb.put(n5 * chunkSize + 1, 1)
  tree.recalculateHash(Set(n5))
  tree.showTree()
}
