package fluence.btree.hash

import java.security.MessageDigest
import JdkCryptoHash._

/**
 * Thread-safe implementation of [[fluence.btree.hash.CryptoHash]] with standard jdk [[java.security.MessageDigest]]
 * @param algorithm one of allowed hashing algorithms
 *                  [[https://docs.oracle.com/javase/7/docs/technotes/guides/security/StandardNames.html#MessageDigest]]
 */
class JdkCryptoHash(algorithm: String) extends CryptoHash[Message, Hash] {

  override def hash(msg1: Message): Hash = {
    MessageDigest.getInstance(algorithm).digest(msg1)
  }

  override def hash(msg1: Message, msg2: Message*): Hash = {
    MessageDigest.getInstance(algorithm).digest(msg1 ++ msg2.flatten)
  }

}

object JdkCryptoHash {

  type Message = Array[Byte]
  type Hash = Array[Byte]

  def apply(algorithm: String): JdkCryptoHash = new JdkCryptoHash(algorithm)

  def sha256(): JdkCryptoHash = apply("SHA-256")

}