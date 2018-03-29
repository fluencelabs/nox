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

package fluence

import cats.Monad
import cats.data.EitherT
import fluence.crypto.algorithm.CryptoErr
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{Signature, Signer}
import scodec.bits.ByteVector

import scala.language.higherKinds

package object contract {

  implicit class EitherTValueReader[E, V](origin: EitherT[Option, E, V]) {

    def success: V =
      origin.value.get.right.get

    def failed: E =
      origin.value.get.left.get
  }

  val signerWithException: Signer = new Signer {
    override def sign[F[_]: Monad](plain: ByteVector): EitherT[F, CryptoErr, Signature] =
      EitherT.leftT(CryptoErr("FAIL"))
    override def publicKey: KeyPair.Public = ???
    override def toString: String = ???
  }

}
