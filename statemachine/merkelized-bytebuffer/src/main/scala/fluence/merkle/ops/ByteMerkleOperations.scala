package fluence.merkle.ops

class ByteMerkleOperations(hashFunc: Array[Byte] => Array[Byte]) extends MerkleOperations[Array[Byte]] {

  override def hash(t: Array[Byte]): Array[Byte] = hashFunc(t)

  override def concatenate(l: Array[Byte], r: Array[Byte]): Array[Byte] = l ++ r

  override def defaultLeaf(chunkSize: Int): Array[Byte] = Array.fill(chunkSize)(0)
}
