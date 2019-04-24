package fluence.merkle

import java.util

import cats.Show
import fluence.merkle.ops.{ByteMerkleOperations, MerkleOperations}
import fluence.merkle.storage.{ByteBufferWrapper, Storage}

class StringBufferOperations extends MerkleOperations[String] {

  override def hash(t: String): String = t

  override def concatenate(l: String, r: String): String = l + r

  override def defaultLeaf(chunkSize: Int): String = "0" * chunkSize
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

  def getDirtyChunks: util.BitSet = {
    affectedChunks
  }

  def put(v: String, start: Int, end: Int): Unit = {
    sb.replace(start, end, v)
    affectedChunks.set(start / chunkSize, end / chunkSize)
  }
}

object TestUtils {

  def initTestMerkle(
    size: Int,
    chunkSize: Int
  ): (StringBufferStorage, BinaryMerkleTree[String]) = {

    val storage: StringBufferStorage = StringBufferStorage(size, chunkSize)
    val operations: StringBufferOperations = new StringBufferOperations()

    implicit val s: Show[String] = Show.fromToString

    val tree = BinaryMerkleTree[String](size, chunkSize, operations, storage)
    println(tree.allNodes.mkString(", "))
    tree.recalculateAll()

    (storage, tree)
  }

  def initBytesTestMerkle(
    size: Int,
    chunkSize: Int,
    hashFunc: Array[Byte] => Array[Byte] = identity
  ): (ByteBufferWrapper, BinaryMerkleTree[Array[Byte]]) = {

    val storage = ByteBufferWrapper.allocate(size, chunkSize)

    val tree = BinaryMerkleTree(size, chunkSize, storage, hashFunc)

    (storage, tree)
  }
}
