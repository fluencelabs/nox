package java.nio

class MerkelizedByteBuffer(cap: Int) extends java.nio.DirectByteBuffer(cap) {
  override def put(i: Int, x: Byte): ByteBuffer = {
    // hook
    super.put(i, x)
  }
}

object TestSome extends App {
  // ByteBuffer size
  val size = 120

  // size of one chunk to hash
  val chunkSize = 10

  // number of chunks with one parent
  val childrenCount = 3

  val childrenBulkCount = size / childrenCount + (if (size % childrenCount == 0) 0 else 1)

  val leafNumber = childrenBulkCount * childrenCount

  println("bulk number: " + childrenBulkCount)
  println("leaf number: " + leafNumber)
}
