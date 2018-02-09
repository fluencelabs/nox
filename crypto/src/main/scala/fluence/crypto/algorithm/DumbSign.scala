package fluence.crypto.algorithm

import java.security.SecureRandom

import cats.Applicative
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import scodec.bits.ByteVector

import scala.language.higherKinds

class DumbSign[F[_]](implicit F: Applicative[F]) extends KeyGenerator[F] with SignatureFunctions[F] {
  override def generateKeyPair(random: SecureRandom): F[KeyPair] = F.pure(KeyPair.fromBytes(random.generateSeed(10), random.generateSeed(10)))

  override def generateKeyPair(): F[KeyPair] = generateKeyPair(new SecureRandom())

  override def sign(keyPair: KeyPair, message: ByteVector): F[Signature] = F.pure(Signature(keyPair.publicKey, message.reverse))

  override def verify(signature: Signature, message: ByteVector): F[Boolean] = F.pure(signature.sign == message.reverse)
}
