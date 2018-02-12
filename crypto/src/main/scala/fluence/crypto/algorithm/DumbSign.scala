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

package fluence.crypto.algorithm

import cats.MonadError
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import scodec.bits.ByteVector

import scala.language.higherKinds

class DumbSign extends KeyGenerator with SignatureFunctions {
  override def generateKeyPair[F[_]](seed: Option[Array[Byte]] = None)(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    val s = seed.getOrElse(Array[Byte](1, 2, 3, 4, 5))
    F.pure(KeyPair.fromBytes(s, s))
  }

  override def sign[F[_]](keyPair: KeyPair, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Signature] =
    F.pure(Signature(keyPair.publicKey, message.reverse))

  override def verify[F[_]](signature: Signature, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Boolean] =
    F.pure(signature.sign == message.reverse)
}
