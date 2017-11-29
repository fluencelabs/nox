package fluence.hash

import java.security.MessageDigest

/**
 * Thread-safe implementation of [[CryptoHasher]] with standard jdk [[java.security.MessageDigest]]
 *
 * @param algorithm one of allowed hashing algorithms
 *                  [[https://docs.oracle.com/javase/7/docs/technotes/guides/security/StandardNames.html#MessageDigest]]
 */
class JdkCryptoHasher(algorithm: String) extends CryptoHasher[Array[Byte], Array[Byte]] {

  override def hash(msg1: Array[Byte]): Array[Byte] = {
    MessageDigest.getInstance(algorithm).digest(msg1)
  }

  override def hash(msg1: Array[Byte], msg2: Array[Byte]*): Array[Byte] = {
    hash(msg1 ++ msg2.flatten)
  }

}

object JdkCryptoHasher {

  lazy val Sha256 = apply("SHA-256")

  def apply(algorithm: String): JdkCryptoHasher = new JdkCryptoHasher(algorithm)

}
