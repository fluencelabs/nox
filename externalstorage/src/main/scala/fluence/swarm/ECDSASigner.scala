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

package fluence.swarm
import fluence.crypto.{Crypto, CryptoError}
import org.web3j.crypto.{ECKeyPair, Sign}
import scodec.bits.ByteVector

import scala.util.Try

object ECDSASigner {

  type Signer[A, B] = Crypto.Func[A, B]

  import fluence.swarm.helpers.SignatureDataOps._

  /**
   * Default sign algorithm in Swarm.
   */
  def signer(kp: ECKeyPair): Signer[ByteVector, ByteVector] =
    Crypto.liftFuncEither(
      bytes ⇒
        Try {
          val signData = Sign.signMessage(bytes.toArray, kp, false)
          ByteVector(signData.toByteArray)
        }.toEither.left
          .map(err ⇒ CryptoError(s"Unexpected error when signing by ECDSA alghorithm.", Some(err)))
    )
}
