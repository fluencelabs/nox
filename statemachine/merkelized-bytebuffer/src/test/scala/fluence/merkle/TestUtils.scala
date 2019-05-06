package fluence.merkle

import fluence.merkle.storage.ByteBufferWrapper

object TestUtils {

  def initBytesTestMerkle(
    size: Int,
    chunkSize: Int,
    hashFunc: Array[Byte] => Array[Byte] = identity
  ): (ByteBufferWrapper, BinaryMerkleTree) = {

    val storage = ByteBufferWrapper.allocate(size, chunkSize)

    val tree = BinaryMerkleTree(size, chunkSize, storage, hashFunc)

    (storage, tree)
  }
}
