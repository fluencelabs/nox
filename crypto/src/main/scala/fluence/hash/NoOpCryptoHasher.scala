package fluence.hash

/**
 * No operation implementation. Do nothing, just return the same value.
 */
object NoOpCryptoHasher extends CryptoHasher[Array[Byte], Array[Byte]] {

  override def hash(msg: Array[Byte]): Array[Byte] = msg

  override def hash(msg1: Array[Byte], msgN: Array[Byte]*): Array[Byte] = msg1 ++ msgN.flatten
}
