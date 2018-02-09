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

package fluence.crypto.signature

import cats.MonadError
import fluence.crypto.keypair.KeyPair
import scodec.bits.ByteVector

import scala.language.higherKinds

trait Signer[F[_]] {
  def publicKey: KeyPair.Public

  def sign(plain: ByteVector): F[Signature]
}

object Signer {
  class DumbSigner[F[_]](keyPair: KeyPair)(implicit F: MonadError[F, Throwable]) extends Signer[F] {
    override def publicKey: KeyPair.Public = keyPair.publicKey

    override def sign(plain: ByteVector): F[Signature] =
      F.pure(Signature(keyPair.publicKey, plain.reverse))
  }
}
