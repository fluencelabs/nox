package fluence.crypto.algorithm

import java.security.SecureRandom

import cats.Monad
import cats.data.EitherT
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import scodec.bits.ByteVector

import scala.language.higherKinds

class DumbSign extends KeyGenerator with SignatureFunctions {
  override def generateKeyPair[F[_] : Monad](random: SecureRandom): EitherT[F, CryptoErr, KeyPair] =
    EitherT.pure(KeyPair.fromBytes(random.generateSeed(10), random.generateSeed(10)))

  override def generateKeyPair[F[_] : Monad](): EitherT[F, CryptoErr, KeyPair] =
    generateKeyPair(new SecureRandom())

  override def sign[F[_] : Monad](keyPair: KeyPair, message: ByteVector): EitherT[F, CryptoErr, Signature] =
    EitherT.pure(Signature(keyPair.publicKey, message.reverse))

  override def verify[F[_] : Monad](signature: Signature, message: ByteVector): EitherT[F, CryptoErr, Unit] =
    EitherT.cond[F](signature.sign == message.reverse, (), CryptoErr("Invalid Signature"))
}
