package fluence.merkle

import java.nio.ByteBuffer
import java.security.MessageDigest

import cats.Show
import scodec.bits.ByteVector

import scala.collection.mutable

/*object Hash {

  val sha256 = MessageDigest.getInstance("SHA-256")

  def apply[A](implicit instance: Hash[A]): Hash[A] = instance

  implicit def hashString: Hash[String] =
    new Hash[String] {
      override def hash(t: String): String = t
    }

  implicit def hashBytes: Hash[Array[Byte]] =
    new Hash[Array[Byte]] {
      override def hash(t: Array[Byte]): Array[Byte] = sha256.digest(t)
    }
}*/

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

  def power2(power: Int): Int = {
    1 << power
  }
}

case class StringHash(hash: String, numberOfTouches: Int) {
  override def toString: String = hash
}

object TestSome extends App {

  /*// ByteBuffer size
  val size = 31

  // size of one chunk to hash
  val chunkSize = 1

  implicit val showDep: Show[String] = Show.fromToString

  val init: Int => StringBuffer = { bbSize =>
    val array = Array.fill[Byte](bbSize)(0)
    println("INIT! size: " + bbSize)

    new StringBuffer("0" * bbSize)
  }

  val getBytes: (StringBuffer, Int, Int) => String = (bb: StringBuffer, offset: Int, length: Int) => {
    val bb1 = new StringBuffer("0" * length)
    println("offset getBytes: " + offset)
    println("length getBytes: " + length)
    val res =
      if (bb.length() <= offset) bb1.toString
      else {
        if (bb.length() >= offset + length) bb.substring(offset, offset + length)
        else bb.substring(offset, bb.length()) + bb1.substring(0, bb.length() - offset)
      }
    println("res " + res + " res")
    res
  }

  val affectedSet = mutable.Set.empty[Int]

  val getAffectedChunks: StringBuffer => Set[Int] = (bb: StringBuffer) => {
    val result = affectedSet.toSet
    affectedSet.clear()
    result
  }

  val (bb, tree) = MerkleTree[StringBuffer, String](size, chunkSize, init, "0", getBytes, getAffectedChunks)
  println(tree.nodes.mkString(", "))
  tree.recalculateAll()

  def put(start: Int, end: Int, str: String) = {
    bb.replace(start, end, str)
    affectedSet.add(start)
  }

  println()
  println("====================================================================")
  println()
  tree.showTree()
  println()
  println("====================================================================")
  println()
  put(1, 2, "1")
  put(4, 5, "2")
  put(7, 8, "3")

  tree.recalculateHash()
  tree.showTree()

  put(7, 8, "4")
  tree.recalculateHash()
  tree.showTree()

  put(14, 15, "7")
  tree.recalculateHash()
  tree.showTree()*/
}
