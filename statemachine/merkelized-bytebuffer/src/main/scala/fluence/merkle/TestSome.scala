package fluence.merkle

import java.nio.ByteBuffer
import java.security.MessageDigest

import cats.Show
import scodec.bits.ByteVector

trait Hash[T] {
  def hash(t: T): T
}

object Hash {

  val sha256 = MessageDigest.getInstance("SHA-256")

  def apply[A](implicit instance: Hash[A]): Hash[A] = instance

  implicit def hashString: Hash[String] =
    new Hash[String] {
      override def hash(t: String): String = "enc(" + t + ")"
    }

  implicit def hashBytes: Hash[Array[Byte]] =
    new Hash[Array[Byte]] {
      override def hash(t: Array[Byte]): Array[Byte] = sha256.digest(t)
    }
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

object TestSome extends App {

  // ByteBuffer size
  val size = 13

  // size of one chunk to hash
  val chunkSize = 2

  implicit val showDep: Show[String] = Show.fromToString

  val init: Int => ByteBufferWrapper = { bbSize =>
    val array = Array.fill[Byte](bbSize)(0)
    println("INIT! size: " + bbSize)
    new ByteBufferWrapper(ByteBuffer.wrap(array), chunkSize)
  }

  val getBytes: (ByteBufferWrapper, Int, Int) => String = (bb: ByteBufferWrapper, offset: Int, length: Int) => {
    val array = new Array[Byte](length)
    bb.position(offset)
    val remaining = bb.remaining()
    val lengthToGet = if (remaining < length) remaining else length
    bb.get(array, 0, lengthToGet)
    ByteVector(array).toHex
  }

  val (bb, tree) = MerkleTree[ByteBufferWrapper, String](size, chunkSize, init, "0000", getBytes)
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
  tree.recalculateHash(bb.getTouchedAndReset())
  tree.showTree()

  val n4 = 0
  bb.put(n4 * chunkSize + 1, 1)
  tree.recalculateHash(bb.getTouchedAndReset())
  tree.showTree()

  val n5 = 1
  bb.put(n5 * chunkSize + 1, 1)
  tree.recalculateHash(bb.getTouchedAndReset())
  tree.showTree()
}
