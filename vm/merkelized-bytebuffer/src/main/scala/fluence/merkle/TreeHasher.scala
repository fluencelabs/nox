package fluence.merkle

import java.nio.ByteBuffer
import java.security.MessageDigest

/**
 * Represents hash calculation for BinayMerkleTree.
 */
trait TreeHasher {
  def digest(arr: Array[Byte]): Array[Byte]
  def digest(bb: ByteBuffer): Array[Byte]
}

object TreeHasher {
  def apply(digester: MessageDigest): TreeHasher = new DigesterTreeHasher(digester)
}

/**
 * Hasher that does nothing and returns input. For test purpose only.
 */
private[this] class PlainTreeHasher extends TreeHasher {
  override def digest(arr: Array[Byte]): Array[Byte] = arr

  override def digest(bb: ByteBuffer): Array[Byte] = {
    val remaining = bb.remaining()
    val arr = new Array[Byte](remaining)
    bb.get(arr)
    arr
  }
}

/**
 * Uses standard JVM digester.
 *
 * @param digester provides applications the functionality of a message digest algorithm
 *
 */
class DigesterTreeHasher(digester: MessageDigest) extends TreeHasher {
  override def digest(arr: Array[Byte]): Array[Byte] = digester.digest(arr)

  override def digest(bb: ByteBuffer): Array[Byte] = {
    digester.update(bb)
    digester.digest()
  }
}
