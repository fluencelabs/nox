package fluence.hash

import java.security.MessageDigest
import JdkCryptoHash._

/**
 * Thread-safe implementation of [[CryptoHash]] with standard jdk [[java.security.MessageDigest]]
 * @param algorithm one of allowed hashing algorithms
 *                  [[https://docs.oracle.com/javase/7/docs/technotes/guides/security/StandardNames.html#MessageDigest]]
 */
class JdkCryptoHash(algorithm: String) extends CryptoHash[Message, Hash] {

  override def hash(msg1: Message): Hash = {
    Hash(MessageDigest.getInstance(algorithm).digest(msg1.origin))
  }

  override def hash(msg1: Message, msg2: Message*): Hash = {
    Hash(MessageDigest.getInstance(algorithm).digest(msg1.origin ++ msg2.flatMap(_.origin)))
  }

}

object JdkCryptoHash {

  lazy val Sha256 = apply("SHA-256")

  case class Message(origin: Array[Byte]) extends AnyVal {
    override def toString: String = new String(origin)
  }
  case class Hash(origin: Array[Byte]) extends AnyVal {
    override def toString: String = new String(origin)
  }

  def apply(algorithm: String): JdkCryptoHash = new JdkCryptoHash(algorithm)

}