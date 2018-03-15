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

  lazy val Sha256: CryptoHasher.Bytes =
    CryptoHasher.buildM[Array[Byte], Array[Byte]] { msg ⇒
      val sha256 = new SHA256()
      sha256.update(msg.toJSArray)
      ByteVector.fromValidHex(sha256.digest("hex")).toArray
    }(_ ++ _)

  lazy val Sha1: CryptoHasher.Bytes =
    CryptoHasher.buildM[Array[Byte], Array[Byte]] { msg ⇒
      val sha1 = new SHA1()
      sha1.update(msg.toJSArray)
      ByteVector.fromValidHex(sha1.digest("hex")).toArray
    }(_ ++ _)
}
