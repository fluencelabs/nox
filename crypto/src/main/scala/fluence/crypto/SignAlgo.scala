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

package fluence.crypto

import java.security.SecureRandom

import cats.MonadError
import fluence.crypto.algorithm.{ DumbSign, KeyGenerator, SignatureFunctions }
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{ Signature, SignatureChecker, Signer }
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Class for generation keys, signers and checkers
 * @param algo implementation of sign alghoritms, e.g. ECDSA
 */
class SignAlgo(algo: KeyGenerator with SignatureFunctions) {

  def generateKeyPair[F[_]]()(implicit F: MonadError[F, Throwable]): F[KeyPair] = algo.generateKeyPair()
  def generateKeyPair[F[_]](seed: ByteVector)(implicit F: MonadError[F, Throwable]): F[KeyPair] = algo.generateKeyPair(new SecureRandom(seed.toArray))

  def signer[F[_]](kp: KeyPair)(implicit F: MonadError[F, Throwable]): Signer[F] = new Signer[F] {
    override def sign(plain: ByteVector): F[Signature] = algo.sign(kp, plain)
    override def publicKey: KeyPair.Public = kp.publicKey
  }

  def checker[F[_]](implicit F: MonadError[F, Throwable]): SignatureChecker[F] = (signature: Signature, plain: ByteVector) ⇒ algo.verify(signature, plain)
}

object SignAlgo {
  val dumb = new SignAlgo(new DumbSign())
}
