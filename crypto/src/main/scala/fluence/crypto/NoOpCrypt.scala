package fluence.crypto

import java.nio.ByteBuffer

/**
 * No operation implementation. Just convert the element to bytes back and forth without any cryptography.
 */
class NoOpCrypt[T](serializer: T ⇒ Array[Byte], deserializer: Array[Byte] ⇒ T) extends Crypt[T, Array[Byte]] {

  def encrypt(plainText: T): Array[Byte] = serializer(plainText)

  def decrypt(cipherText: Array[Byte]): T = deserializer(cipherText)

}

object NoOpCrypt {

  val forString: NoOpCrypt[String] = apply[String](
    serializer = _.getBytes,
    deserializer = bytes ⇒ new String(bytes))

  val forLong: NoOpCrypt[Long] = apply[Long](
    serializer = ByteBuffer.allocate(java.lang.Long.BYTES).putLong(_).array(),
    deserializer = bytes ⇒ ByteBuffer.wrap(bytes).getLong()
  )

  def apply[T](serializer: T ⇒ Array[Byte], deserializer: Array[Byte] ⇒ T): NoOpCrypt[T] =
    new NoOpCrypt(serializer, deserializer)

}
