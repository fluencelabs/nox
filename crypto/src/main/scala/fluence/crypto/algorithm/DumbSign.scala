package fluence.crypto.algorithm

import cats.MonadError
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import scodec.bits.ByteVector

import scala.language.higherKinds

class DumbSign extends KeyGenerator with SignatureFunctions {
  override def generateKeyPair[F[_]](seed: Option[Array[Byte]] = None)(implicit F: MonadError[F, Throwable]): F[KeyPair] = {
    val s = seed.getOrElse(Array[Byte](1, 2, 3, 4, 5))
    F.pure(KeyPair.fromBytes(s, s))
  }

  override def sign[F[_]](keyPair: KeyPair, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Signature] =
    F.pure(Signature(keyPair.publicKey, message.reverse))

  override def verify[F[_]](signature: Signature, message: ByteVector)(implicit F: MonadError[F, Throwable]): F[Boolean] =
    F.pure(signature.sign == message.reverse)
}
