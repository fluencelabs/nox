package fluence.merkle

import java.nio.ByteBuffer

object TestUtils {

  def initBytesTestMerkle(
    size: Int,
    chunkSize: Int,
    direct: Boolean = false,
    hashFuncLeafs: ByteBuffer => Array[Byte] = { bb =>
      val arr = new Array[Byte](bb.limit() - bb.position())
      bb.get(arr)
      arr
    },
    hashFuncNodes: Array[Byte] => Array[Byte] = identity
  ): (TrackingMemoryBuffer, BinaryMerkleTree) = {

    val storage =
      if (direct) TrackingMemoryBuffer.allocateDirect(size, chunkSize)
      else TrackingMemoryBuffer.allocate(size, chunkSize)

    val tree = BinaryMerkleTree(chunkSize, hashFuncLeafs, hashFuncNodes, storage)

    (storage, tree)
  }
}
