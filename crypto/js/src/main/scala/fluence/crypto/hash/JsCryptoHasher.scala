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

import fluence.crypto.facade.ecdsa.{ SHA1, SHA256 }
import scodec.bits.ByteVector

import scala.scalajs.js.JSConverters._

object JsCryptoHasher {

  lazy val Sha256: CryptoHasher.Bytes = new CryptoHasher[Array[Byte], Array[Byte]] {
    override def hash(msg1: Array[Byte]): Array[Byte] = {
      val sha256 = new SHA256()
      sha256.update(msg1.toJSArray)
      ByteVector.fromValidHex(sha256.digest("hex")).toArray
    }
    override def hash(msg1: Array[Byte], msg2: Array[Byte]*): Array[Byte] = {
      hash(msg1 ++ msg2.flatten)
    }
  }

  lazy val Sha1: CryptoHasher.Bytes = new CryptoHasher[Array[Byte], Array[Byte]] {
    override def hash(msg1: Array[Byte]): Array[Byte] = {
      val sha1 = new SHA1()
      sha1.update(msg1.toJSArray)
      ByteVector.fromValidHex(sha1.digest("hex")).toArray
    }
    override def hash(msg1: Array[Byte], msg2: Array[Byte]*): Array[Byte] = {
      hash(msg1 ++ msg2.flatten)
    }

  }
}
