package fluence.btree

import java.nio.ByteBuffer

import scala.math.Ordering

package object index {

  type Key = Array[Byte]
  type Value = Array[Byte]

  implicit object BytesOrdering extends Ordering[Array[Byte]] {
    override def compare(x: Array[Byte], y: Array[Byte]): Int = ByteBuffer.wrap(x).compareTo(ByteBuffer.wrap(y))
  }

}
