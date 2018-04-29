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

import fluence.crypto.{Crypto, CryptoError}
import fluence.crypto.facade.ecdsa.{SHA1, SHA256}
import scodec.bits.ByteVector

import scala.scalajs.js.JSConverters._
import scala.scalajs.js.typedarray.Uint8Array
import scala.util.Try

object JsCryptoHasher {

  lazy val Sha256: Crypto.Hasher[Array[Byte], Array[Byte]] =
    Crypto.liftFuncEither[Array[Byte], Array[Byte]] { msg ⇒
      Try {
        val sha256 = new SHA256()
        sha256.update(new Uint8Array(msg.toJSArray))
        ByteVector.fromValidHex(sha256.digest("hex")).toArray
      }.toEither.left.map(err ⇒ CryptoError("Cannot calculate Sha256 hash", Some(err)))
    }

  lazy val Sha1: Crypto.Hasher[Array[Byte], Array[Byte]] =
    Crypto.liftFuncEither[Array[Byte], Array[Byte]] { msg ⇒
      Try {
        val sha1 = new SHA1()
        sha1.update(new Uint8Array(msg.toJSArray))
        ByteVector.fromValidHex(sha1.digest("hex")).toArray
      }.toEither.left.map(err ⇒ CryptoError("Cannot calculate Sha256 hash", Some(err)))
    }
}
