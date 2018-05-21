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

import cats.Monad
import cats.data.EitherT
import cats.syntax.strong._
import cats.syntax.compose._
import fluence.crypto.{Crypto, CryptoError, KeyPair}
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Signature algorithm -- cryptographically coupled keypair, signer and signature checker.
 *
 * @param name Algorithm name
 * @param generateKeyPair Keypair generator; note that you must ensure the seed entropy is secure
 * @param signer Signer for a given keypair
 * @param checker Checker for a given public key
 */
case class SignAlgo(
  name: String,
  generateKeyPair: Crypto.KeyPairGenerator,
  signer: SignAlgo.SignerFn,
  implicit val checker: SignAlgo.CheckerFn,
)

object SignAlgo {
  type SignerFn = KeyPair ⇒ Signer

  type CheckerFn = KeyPair.Public ⇒ SignatureChecker

  /**
   * Take checker, signature, and plain data, and apply checker, returning Unit on success, or left side error.
   */
  private val fullChecker: Crypto.Func[((SignatureChecker, Signature), ByteVector), Unit] =
    new Crypto.Func[((SignatureChecker, Signature), ByteVector), Unit] {
      override def apply[F[_]: Monad](
        input: ((SignatureChecker, Signature), ByteVector)
      ): EitherT[F, CryptoError, Unit] = {
        val ((signatureChecker, signature), plainData) = input
        signatureChecker.check(signature, plainData)
      }
    }

  /**
   * For CheckerFn, builds a function that takes PubKeyAndSignature along with plain data, and checks the signature.
   */
  def checkerFunc(fn: CheckerFn): Crypto.Func[(PubKeyAndSignature, ByteVector), Unit] =
    Crypto
      .liftFunc[PubKeyAndSignature, (SignatureChecker, Signature)] {
        case PubKeyAndSignature(pk, signature) ⇒ fn(pk) -> signature
      }
      .first[ByteVector] andThen fullChecker

}
