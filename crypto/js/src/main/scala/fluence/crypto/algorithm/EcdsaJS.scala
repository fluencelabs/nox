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

import cats.data.EitherT
import cats.Monad
import fluence.crypto.SignAlgo
import fluence.crypto.facade.ecdsa.EC
import fluence.crypto.hash.{ CryptoHasher, JsCryptoHasher }
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.scalajs.js
import scala.scalajs.js.JSConverters._

/**
 * Return in all js methods hex, because in the other case we will receive javascript objects
 * @param ec implementation of ecdsa logic for different curves
 */
class EcdsaJS(ec: EC, hasher: Option[CryptoHasher[Array[Byte], Array[Byte]]]) extends Algorithm with SignatureFunctions with KeyGenerator {
  import CryptoErr._

  override def generateKeyPair[F[_] : Monad](seed: Option[Array[Byte]] = None): EitherT[F, CryptoErr, KeyPair] = {
    nonFatalHandling {
      val seedJs = seed.map(bs ⇒ js.Dynamic.literal(entropy = bs.toJSArray))
      val key = ec.genKeyPair(seedJs)
      val publicHex = key.getPublic(true, "hex")
      val secretHex = key.getPrivate("hex")
      val public = ByteVector.fromValidHex(publicHex)
      val secret = ByteVector.fromValidHex(secretHex)
      KeyPair.fromByteVectors(public, secret)
    } ("Failed to generate key pair.")
  }

  override def sign[F[_] : Monad](keyPair: KeyPair, message: ByteVector): EitherT[F, CryptoErr, Signature] = {
    for {
      secret ← nonFatalHandling{
        ec.keyFromPrivate(keyPair.secretKey.value.toHex, "hex")
      }("Cannot get private key from key pair.")
      hash ← hash(message)
      signHex ← nonFatalHandling(secret.sign(hash).toDER("hex"))("Cannot sign message")
    } yield Signature(keyPair.publicKey, ByteVector.fromValidHex(signHex))
  }

  def hash[F[_] : Monad](message: ByteVector): EitherT[F, CryptoErr, js.Array[Byte]] = {
    val arr = message.toArray
    hasher.fold(EitherT.pure[F, CryptoErr](arr)) { h ⇒
      nonFatalHandling {
        h.hash(message.toArray)
      }("Cannot hash message.")
    }.map(_.toJSArray)
  }

  override def verify[F[_] : Monad](signature: Signature, message: ByteVector): EitherT[F, CryptoErr, Unit] = {
    for {
      public ← nonFatalHandling{
        val hex = signature.publicKey.value.toHex
        ec.keyFromPublic(hex, "hex")
      }("Incorrect public key format.")
      hash ← hash(message)
      verify ← nonFatalHandling(public.verify(hash, signature.sign.toHex))("Cannot verify message.")
      _ ← EitherT.cond[F](verify, (), CryptoErr("Signature is not verified"))
    } yield ()
  }

}

object EcdsaJS {
  val ecdsa_secp256k1_sha256 = new EcdsaJS(new EC("secp256k1"), Some(JsCryptoHasher.Sha256))

  val signAlgo = new SignAlgo("ecdsa/secp256k1/sha256/js", ecdsa_secp256k1_sha256)
}
