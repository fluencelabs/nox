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

import cats.Monad
import cats.data.EitherT
import fluence.crypto.signature.{SignAlgo, Signature, SignatureChecker, Signer}
import scodec.bits.ByteVector

import scala.language.higherKinds

object DumbCrypto {

  lazy val signAlgo: SignAlgo =
    SignAlgo(
      "dumb",
      Crypto.liftFunc { seedOpt ⇒
        val seed = seedOpt.getOrElse {
          new SecureRandom().generateSeed(32)
        }
        KeyPair.fromBytes(seed, seed)
      },
      keyPair ⇒ Signer(keyPair.publicKey, Crypto.liftFunc(plain ⇒ Signature(plain.reverse))),
      publicKey ⇒
        new SignatureChecker {
          override def check[F[_]: Monad](signature: Signature, plain: ByteVector): EitherT[F, CryptoError, Unit] =
            EitherT.cond[F](signature.sign == plain.reverse, (), CryptoError("Signatures mismatch"))
      }
    )

  lazy val cipherString: Crypto.Cipher[String] =
    Crypto.liftB(_.getBytes, bytes ⇒ new String(bytes))

  lazy val noOpHasher: Crypto.Hasher[Array[Byte], Array[Byte]] =
    Crypto.identityFunc[Array[Byte]]

  lazy val testHasher: Crypto.Hasher[Array[Byte], Array[Byte]] =
    Crypto.liftFunc(bytes ⇒ ("H<" + new String(bytes) + ">").getBytes())
}
