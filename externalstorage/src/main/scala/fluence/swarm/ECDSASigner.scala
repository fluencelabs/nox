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
