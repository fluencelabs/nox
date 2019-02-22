/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.swarm.crypto
import fluence.crypto.{Crypto, CryptoError}
import org.web3j.crypto.Hash
import scodec.bits.ByteVector

import scala.util.Try

object Keccak256Hasher {

  /**
   * Default hash function in Swarm. Arrow from plain bytes to hash bytes.
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
