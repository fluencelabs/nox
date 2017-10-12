package fluence.kad

import java.util.Base64

import cats.{ Monoid, Order, Show }
import cats.syntax.monoid._

/**
 * Kademlia Key is 160 bits (sha-1 length) in byte array.
 * We use value case class for type safety, and typeclasses for ops.
 *
 * @param id ID
 */
final case class Key(id: Array[Byte]) extends AnyVal {
  /**
   * Number of leading zeros
   *
   * @return
   */
  def zerosPrefixLen: Int = {
    val idx = id.indexWhere(_ != 0)
    if (idx < 0) {
      Key.BitLength
    } else {
      Integer.numberOfLeadingZeros(id(idx)) + 8 * (idx - 3)
    }
  }
}

object Key {
  val Length = 20
  val BitLength: Int = Length * 8

  // XOR Monoid is used for Kademlia distance
  implicit object XorDistanceMonoid extends Monoid[Key] {
    override val empty: Key = Key(Array.ofDim[Byte](Length)) // filled with zeros

    override def combine(x: Key, y: Key): Key = Key {
      var i = 0
      val ret = Array.ofDim[Byte](Length)
      while (i < Length) {
        ret(i) = (x.id(i) ^ y.id(i)).toByte
        i += 1
      }
      ret
    }
  }

  // Kademlia keys are ordered, low order byte is the most significant
  implicit object OrderedKeys extends Order[Key] {
    override def compare(x: Key, y: Key): Int = {
      var i = 0
      while (i < Length) {
        if (x.id(i) != y.id(i)) {
          return x.id(i).abs compareTo y.id(i).abs
        }
        i += 1
      }
      0
    }
  }

  // Order relative to a distinct key
  def relativeOrder(key: Key): Order[Key] =
    (x, y) ⇒ OrderedKeys.compare(x |+| key, y |+| key)

  def relativeOrdering(key: Key): Ordering[Key] =
    relativeOrder(key).compare(_, _)

  implicit object ShowKeyBase64 extends Show[Key] {
    override def show(f: Key): String =
      Base64.getEncoder.encodeToString(f.id)
  }

}
