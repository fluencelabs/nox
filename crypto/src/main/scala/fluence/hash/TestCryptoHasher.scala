package fluence.hash

/** For testing and debugging. The result is human readable. */
object TestCryptoHasher extends CryptoHasher[Array[Byte], Array[Byte]] {

  override def hash(msg: Array[Byte]): Array[Byte] = {
    ("H<" + new String(msg) + ">").getBytes()
  }

  override def hash(msg1: Array[Byte], msgN: Array[Byte]*): Array[Byte] = {
    hash(msg1 ++ msgN.flatten)
  }

}
