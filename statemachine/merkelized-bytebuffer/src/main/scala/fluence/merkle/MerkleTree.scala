package fluence.merkle

import cats.Show

import scala.reflect.ClassTag

case class MerkleTree[I, H: Show: Append](
  nodes: Array[H],
  treeHeight: Int,
  struct: I,
  get: (I, Int, Int) => H,
  chunkSize: Int
)(implicit Hash: Hash[H], m: ClassTag[H]) {
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

  private def getRoot: H = {
    nodes(0)
  }

  private def getChildrens(height: Int, pos: Int): Array[H] = {
    if (height == treeHeight) throw new RuntimeException("No childrens on leafs")
    getBatch(height + 1, pos * 2)
  }

  /**
   * Gets node with its neighbor
   */
  private def getBatch(height: Int, pos: Int): Array[H] = {
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
      val childrens = getChildrens(0, 0).map(Hash.hash).toList
      nodes(0) = Hash.hash(Append[H].append(childrens))
      nodes(0)
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
          val newHash = Hash.hash(bytes)
          println("NEW LEAF HASH: " + newHash)
          nodes(index) = newHash
        } else {
          println(s"height: $height, i: $i, index: $index")
          val childrens = getChildrens(height, i).map(Hash.hash).toList
          println(s"childrens: ${childrens.mkString(", ")}")
          val newHash =
            Hash.hash(Append[H].append(childrens))
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

object MerkleTree {

  def apply[I, H: Show: Append](
    size: Int,
    chunkSize: Int,
    init: Int => I,
    default: H,
    get: (I, Int, Int) => H
  )(implicit Hash: Hash[H], m: ClassTag[H]): (I, MerkleTree[I, H]) = {
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

    val struct = init(size)
    val arrayTree = new Array[H](numberOfNodes)
    for (i <- 0 until numberOfNodes) {
      if (i < mappedLeafCount) {
        arrayTree(i) = Hash.hash(get(struct, i * chunkSize, chunkSize))
      } else {
        arrayTree(i) = Hash.hash(default)
      }

    }
    (struct, MerkleTree(arrayTree, treeHeight, struct, get, chunkSize))
  }
}
