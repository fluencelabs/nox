/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.crypto.hash

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

  lazy val Sha256: CryptoHasher[Array[Byte], Array[Byte]] = apply("SHA-256")
  lazy val Sha1: CryptoHasher[Array[Byte], Array[Byte]] = apply("SHA-1")

  def apply(algorithm: String): CryptoHasher[Array[Byte], Array[Byte]] = new JdkCryptoHasher(algorithm)

}
