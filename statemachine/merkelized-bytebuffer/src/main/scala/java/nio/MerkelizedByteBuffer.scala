package java.nio

class MerkelizedByteBuffer(cap: Int) extends java.nio.DirectByteBuffer(cap) {
  override def put(i: Int, x: Byte): ByteBuffer = {
    // hook
    super.put(i, x)
  }
}
