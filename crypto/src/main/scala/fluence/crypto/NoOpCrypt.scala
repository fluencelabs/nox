package fluence.crypto

/**
 * No operation implementation. Do nothing, just return the same value.
 */
object NoOpCrypt extends Crypt[Array[Byte], Array[Byte]] {

  def encrypt(plainText: Array[Byte]): Array[Byte] = plainText

  def decrypt(cipherText: Array[Byte]): Array[Byte] = cipherText

}
