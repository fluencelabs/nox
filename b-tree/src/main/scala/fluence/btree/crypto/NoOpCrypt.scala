package fluence.btree.crypto

import fluence.btree.crypto.NoOpCrypt._

/**
 * No operation implementation. Do nothing, just return the same value.
 */
class NoOpCrypt() extends Crypt[Plain, Cipher] {

  def encrypt(plainText: Plain): Cipher = plainText

  def decrypt(cipherText: Cipher): Plain = cipherText

}

object NoOpCrypt {

  type Plain = Array[Byte]
  type Cipher = Array[Byte]

  def apply(): NoOpCrypt = new NoOpCrypt()

}