package fluence
import java.nio.ByteBuffer

import cats.Show
import scodec.bits.ByteVector

case class Hash[T: Show](hash: T, height: Int, index: Int, numberOfChanges: Int = 0) {
  override def toString: String = s"hash: ${Show[T].show(hash)} ($height, $index, $numberOfChanges)"
}

object Hash {
  def default[T: Default: Show](height: Int, index: Int): Hash[T] = new Hash(Default[T].default, height, index)
}

class MerkleTreeBuilder[A](size: Int, chunkSize: Int) {

  def log2(input: Int): Int = {
    31 - Integer.numberOfLeadingZeros(input)
  }

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
    val k = 1 << power
    if (k >= mappedLeafCount) k
    else 1 << (power + 1)
  }

  val treeHeight = log2(16)

  val numberOfNodes = (0 to treeHeight).fold(0) {
    case (acc, v) =>
      val s = leafCount / (1 << v)
      acc + s
  }

  def fillTree[I, H: Default: Show: Append](
    init: Int => I,
    hashFunc: H => H,
    get: (I, Int, Int) => H
  ): (I, MerkleTree[I, H]) = {
    val arrayTree = new Array[Hash[H]](numberOfNodes)
    for (i <- 0 until numberOfNodes) {
      val height = log2(i + 1)
      arrayTree(i) = Hash.default(height, i)
    }
    val struct = init(size)
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

  private def getNodeIndex(height: Int, pos: Int) = {
    if (height < 0 || height > treeHeight) throw new RuntimeException("invalid height")
    val startOfLine = 1 << height
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
  final def recalculateHash(affectedChunks: Set[Int], height: Int = treeHeight - 1): H = {
    if (height == 0) {
      println(s"height: $height")
      val childrens = getChildrens(0, 0).map(_.hash).toList
      nodes(0) = Hash(hashFunc(Append[H].append(childrens)), height, 0, nodes(0).numberOfChanges + 1)
      nodes(0).hash
    } else {
      val affectedParents: Set[Int] = for (i <- affectedChunks) yield {
        val index = getNodeIndex(height, i)
        if (height == treeHeight - 1) {
          val offset = i * chunkSize
          val length = chunkSize
          println(s"offset: $offset, length: $length")
          println(s"height: $height, i: $i")
          println(s"index: $index")
          val bytes = get(struct, offset, length)
          println("BYTES === " + bytes)
          val newHash = Hash(hashFunc(bytes), height, index, nodes(index).numberOfChanges + 1)
          println("NEW LEAF HASH: " + newHash)
          nodes(index) = newHash
        } else {
          println(s"height: $height, i: $i, index: $index")
          val childrens = getChildrens(height, i).map(_.hash).toList
          println(s"childrens: ${childrens.mkString(", ")}")
          val newHash =
            Hash(hashFunc(Append[H].append(childrens)), height, index, nodes(index).numberOfChanges + 1)
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
    nodes.foldLeft(0) {
      case (height, v) =>
        if (height == v.height) {
          print(v.toString + "| ")
          height
        } else {
          println()
          print(v.toString + "| ")
          height + 1
        }
    }
  }
}

object TestSome extends App {

  // ByteBuffer size
  val size = 12

  // size of one chunk to hash
  val chunkSize = 2

  val treeBuilder = new MerkleTreeBuilder(size, chunkSize)

  println("leaf number: " + treeBuilder.leafCount)
  println("mapped number: " + treeBuilder.mappedLeafCount)

  println("tree height: " + treeBuilder.treeHeight)
  println("tree number of nodes: " + treeBuilder.numberOfNodes)

  import Default._
  implicit val showDep: Show[String] = Show.fromToString

  val init: Int => ByteBuffer = { bbSize =>
    val array = Array.fill[Byte](bbSize)(0)
    println("INIT!")
    ByteBuffer.wrap(array)
  }

  val getBytes: (ByteBuffer, Int, Int) => String = (bb: ByteBuffer, offset: Int, length: Int) => {
    val array = new Array[Byte](length)
    bb.position(offset)
    bb.get(array, 0, length)
    println("ARRAY = " + array.mkString(", "))
    ByteVector(array).toHex
  }

  val (bb, tree) = treeBuilder.fillTree[ByteBuffer, String](init, str => {
    "enc(" + str + ")"
  }, getBytes)
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
