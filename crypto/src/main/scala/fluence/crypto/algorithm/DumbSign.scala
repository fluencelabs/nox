package fluence.crypto.algorithm

import java.security.SecureRandom

import cats.MonadError
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import scodec.bits.ByteVector

import scala.language.higherKinds

class DumbSign extends KeyGenerator with SignatureFunctions {
  override def generateKeyPair[F[_]](random: SecureRandom)(implicit F: MonadError[F, Throwable]): F[KeyPair] =
    F.pure(KeyPair.fromBytes(random.generateSeed(10), random.generateSeed(10)))

  override def generateKeyPair[F[_]]()(implicit F: MonadError[F, Throwable]): F[KeyPair] =
    generateKeyPair(new SecureRandom())

  override def sign[F[_]](keyPair: KeyPair, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Signature] =
    F.pure(Signature(keyPair.publicKey, message.reverse))

  override def verify[F[_]](signature: Signature, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Boolean] =
    F.pure(signature.sign == message.reverse)
}
