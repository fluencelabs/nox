package fluence.crypto

import java.security.SecureRandom

import cats.MonadError
import fluence.crypto.algorithm.{ DumbSign, KeyGenerator, SignatureFunctions }
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{ Signature, SignatureChecker, Signer }
import scodec.bits.ByteVector

import scala.language.higherKinds

class SignAlgo[F[_]](algo: KeyGenerator[F] with SignatureFunctions[F])(implicit F: MonadError[F, Throwable]) {

  def generateKeyPair(): F[KeyPair] = algo.generateKeyPair()
  def generateKeyPair(seed: ByteVector): F[KeyPair] = algo.generateKeyPair(new SecureRandom(seed.toArray))

  def signer(kp: KeyPair): Signer[F] = new Signer[F] {
    override def sign(plain: ByteVector): F[Signature] = algo.sign(kp, plain)
    override def publicKey: KeyPair.Public = kp.publicKey
  }

  val checker: SignatureChecker[F] = (signature: Signature, plain: ByteVector) ⇒ algo.verify(signature, plain)
}

object SignAlgo {
  def dumb[F[_]](implicit F: MonadError[F, Throwable]) = new SignAlgo[F](new DumbSign[F]())
}
