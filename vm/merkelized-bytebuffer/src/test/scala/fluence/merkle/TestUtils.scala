package fluence.merkle

import java.security.MessageDigest

object TestUtils {

  def initBytesTestMerkle(
    size: Int,
    chunkSize: Int,
    direct: Boolean = false,
    digester: Option[MessageDigest] = None
  ): (TrackingMemoryBuffer, BinaryMerkleTree) = {

    val storage =
      if (direct) TrackingMemoryBuffer.allocateDirect(size, chunkSize)
      else TrackingMemoryBuffer.allocate(size, chunkSize)

    val hasher = digester.map(TreeHasher.apply).getOrElse(new PlainTreeHasher)

    val tree = BinaryMerkleTree(hasher, storage)

    (storage, tree)
  }
}
