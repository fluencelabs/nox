/*
 * Copyright (C) 2018  Fluence Labs Limited
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

package fluence.swarm
import fluence.crypto.{Crypto, CryptoError}
import org.web3j.crypto.Hash
import scodec.bits.ByteVector

import scala.util.Try

object Keccak256Hasher {

  /**
   * Default hash function in Swarm.
   */
  val hasher: Crypto.Hasher[ByteVector, ByteVector] =
    Crypto.liftFuncEither(
      bytes ⇒
        Try {
          ByteVector(Hash.sha3(bytes.toArray))
        }.toEither.left
          .map(err ⇒ CryptoError(s"Unexpected error when hashing by SHA-3.", Some(err)))
    )
}
