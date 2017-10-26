package fluence.crypto

/**
 * Base interface for encrypting/decrypting.
 * @tparam P type of plain text, input
 * @tparam C type of cipher text, output
 */
trait Crypt[P, C] {

  def encrypt(plainText: P): C

  def decrypt(cipherText: C): P

}
