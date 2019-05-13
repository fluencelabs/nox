package fluence.merkle

object TestUtils {

  def initBytesTestMerkle(
    size: Int,
    chunkSize: Int,
    hashFunc: Array[Byte] => Array[Byte] = identity
  ): (TrackingMemoryBuffer, BinaryMerkleTree) = {

    val storage = TrackingMemoryBuffer.allocate(size, chunkSize)

    val tree = BinaryMerkleTree(size, chunkSize, hashFunc, storage)

    (storage, tree)
  }
}
