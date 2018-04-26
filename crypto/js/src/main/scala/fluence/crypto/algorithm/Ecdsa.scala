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

import cats.Monad
import cats.data.EitherT
import fluence.crypto._
import fluence.crypto.facade.ecdsa.EC
import fluence.crypto.hash.JsCryptoHasher
import fluence.crypto.signature.{SignAlgo, Signature, SignatureChecker, Signer}
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.scalajs.js
import scala.scalajs.js.JSConverters._
import scala.scalajs.js.typedarray.Uint8Array

/**
 * Return in all js methods hex, because in the other case we will receive javascript objects
 * @param ec implementation of ecdsa logic for different curves
 */
class Ecdsa(ec: EC, hasher: Option[Crypto.Hasher[Array[Byte], Array[Byte]]])
    extends Algorithm with SignatureFunctions with KeyGenerator {
  import CryptoError.nonFatalHandling

  override def generateKeyPair: Crypto.KeyPairGenerator =
    new Crypto.Func[Option[Array[Byte]], KeyPair] {
      override def apply[F[_]](input: Option[Array[Byte]])(implicit F: Monad[F]): EitherT[F, CryptoError, KeyPair] =
        nonFatalHandling {
          val seedJs = input.map(bs ⇒ js.Dynamic.literal(entropy = bs.toJSArray))
          val key = ec.genKeyPair(seedJs)
          val publicHex = key.getPublic(true, "hex")
          val secretHex = key.getPrivate("hex")
          val public = ByteVector.fromValidHex(publicHex)
          val secret = ByteVector.fromValidHex(secretHex)
          KeyPair.fromByteVectors(public, secret)
        }("Failed to generate key pair.")
    }

  override def sign[F[_]: Monad](keyPair: KeyPair, message: ByteVector): EitherT[F, CryptoError, Signature] =
    for {
      secret ← nonFatalHandling {
        ec.keyFromPrivate(keyPair.secretKey.value.toHex, "hex")
      }("Cannot get private key from key pair.")
      hash ← hash(message)
      signHex ← nonFatalHandling(secret.sign(new Uint8Array(hash)).toDER("hex"))("Cannot sign message")
    } yield Signature(ByteVector.fromValidHex(signHex))

  def hash[F[_]: Monad](message: ByteVector): EitherT[F, CryptoError, js.Array[Byte]] = {
    val arr = message.toArray
    hasher
      .fold(EitherT.pure[F, CryptoError](arr)) { h ⇒
        h[F](arr)
      }
      .map(_.toJSArray)
  }

  override def verify[F[_]: Monad](
    pubKey: KeyPair.Public,
    signature: Signature,
    message: ByteVector
  ): EitherT[F, CryptoError, Unit] =
    for {
      public ← nonFatalHandling {
        val hex = pubKey.value.toHex
        ec.keyFromPublic(hex, "hex")
      }("Incorrect public key format.")
      hash ← hash(message)
      verify ← nonFatalHandling(public.verify(new Uint8Array(hash), signature.sign.toHex))("Cannot verify message.")
      _ ← EitherT.cond[F](verify, (), CryptoError("Signature is not verified"))
    } yield ()
}

object Ecdsa {
  val ecdsa_secp256k1_sha256 = new Ecdsa(new EC("secp256k1"), Some(JsCryptoHasher.Sha256))

  val signAlgo: SignAlgo = SignAlgo(
    "ecdsa/secp256k1/sha256/js",
    generateKeyPair = ecdsa_secp256k1_sha256.generateKeyPair,
    signer = kp ⇒
      Signer(
        kp.publicKey,
        new Crypto.Func[ByteVector, signature.Signature] {
          override def apply[F[_]](
            input: ByteVector
          )(implicit F: Monad[F]): EitherT[F, CryptoError, signature.Signature] =
            ecdsa_secp256k1_sha256.sign(kp, input)
        }
    ),
    checker = pk ⇒
      new SignatureChecker {
        override def check[F[_]: Monad](
          signature: fluence.crypto.signature.Signature,
          plain: ByteVector
        ): EitherT[F, CryptoError, Unit] =
          ecdsa_secp256k1_sha256.verify(pk, signature, plain)
    }
  )
}
