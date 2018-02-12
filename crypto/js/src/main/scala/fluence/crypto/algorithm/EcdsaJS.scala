package fluence.crypto.algorithm

import cats.MonadError
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.crypto.facade.EC
import fluence.crypto.hash.JsCryptoHasher
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.scalajs.js
import scala.scalajs.js.JSConverters._

class EcdsaJS(ec: EC) extends Algorithm with SignatureFunctions with KeyGenerator {
  import CryptoErr._

  override def generateKeyPair[F[_]](seed: Option[Array[Byte]] = None)(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    nonFatalHandling {
      val seedJs = seed.map(bb ⇒ js.Dynamic.literal(entropy = bb.toJSArray))
      val key = ec.genKeyPair(seedJs)
      val publicHex = key.getPublic(true, "hex")
      val secretHex = key.getPrivate("hex")
      val public = ByteVector.fromValidHex(publicHex)
      val secret = ByteVector.fromValidHex(secretHex)
      KeyPair.fromByteVectors(public, secret)
    } ("Failed to generate key pair.")
  }

  override def sign[F[_]](keyPair: KeyPair, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Signature] = {
    for {
      secret ← nonFatalHandling{
        ec.keyFromPrivate(keyPair.secretKey.value.toHex, "hex")
      }("Cannot get private key from key pair.")
      hash ← hash(message)
      signHex ← nonFatalHandling(secret.sign(hash).toDER("hex"))("Cannot sign message")
    } yield Signature(keyPair.publicKey, ByteVector.fromValidHex(signHex))
  }

  def hash[F[_]](message: ByteVector)(implicit F: MonadError[F, Throwable]): F[js.Array[Byte]] = {
    nonFatalHandling {
      JsCryptoHasher.Sha256.hash(message.toArray).toJSArray
    }("Cannot hash message.")
  }

  override def verify[F[_]](signature: Signature, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Boolean] = {
    for {
      public ← nonFatalHandling{
        val hex = signature.publicKey.value.toHex
        ec.keyFromPublic(hex, "hex")
      }("Incorrect public key format")
      hash ← hash(message)
      res ← nonFatalHandling(public.verify(hash, signature.sign.toHex))("Cannot verify message.")
    } yield res
  }

}

object EcdsaJS {
  def ecdsa_secp256k1_sha256[F[_]](implicit F: MonadError[F, Throwable]) = new EcdsaJS(new EC("secp256k1"))
}
