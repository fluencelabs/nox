package fluence.crypto.algorithm

import cats.{ Monad, MonadError }
import fluence.codec.Codec
import fluence.crypto.cipher.Crypt

import scala.language.higherKinds

class AesCryptJS[F[_] : Monad, T](implicit ME: MonadError[F, Throwable], codec: Codec[F, T, Array[Byte]]) extends Crypt[F, T, Array[Byte]] {
  override def encrypt(plainText: T): F[Array[Byte]] = ???

  override def decrypt(cipherText: Array[Byte]): F[T] = ???
}
