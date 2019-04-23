package fluence.merkle

object TestUtils {

  def initTestMerkle(
    size: Int,
    chunkSize: Int
  ): (StringBufferStorage, MerkleTree[Storage, String, String]) = {

    implicit val storage: StringBufferStorage = StringBufferStorage(size, chunkSize)
    implicit val operations: StringBufferOperations = new StringBufferOperations()

    val tree = MerkleTree[Storage, String, String](size, chunkSize)
    println(tree.nodes.mkString(", "))
    tree.recalculateAll()

    (storage, tree)
  }
}
