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

import cats.Monad
import cats.data.EitherT
import fluence.crypto.algorithm.{ CryptoErr, DumbSign, KeyGenerator, SignatureFunctions }
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{ Signature, SignatureChecker, Signer }
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Class for generation keys, signers and checkers
 * @param algo implementation of sign alghoritms, e.g. ECDSA
 */
class SignAlgo(algo: KeyGenerator with SignatureFunctions) {

  def generateKeyPair[F[_] : Monad](seed: Option[ByteVector] = None): EitherT[F, CryptoErr, KeyPair] =
    algo.generateKeyPair(seed.map(_.toArray))

  def signer(kp: KeyPair): Signer = new Signer {
    override def sign[F[_] : Monad](plain: ByteVector): EitherT[F, CryptoErr, Signature] = algo.sign(kp, plain)
    override def publicKey: KeyPair.Public = kp.publicKey
  }

  def checker: SignatureChecker = new SignatureChecker {
    override def check[F[_] : Monad](signature: Signature, plain: ByteVector): EitherT[F, CryptoErr, Unit] = algo.verify(signature, plain)
  }
}

object SignAlgo {
  val dumb = new SignAlgo(new DumbSign())
}
