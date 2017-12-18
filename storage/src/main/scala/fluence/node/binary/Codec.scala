package fluence.node.binary

import cats.Applicative
import cats.syntax.applicative._

import scala.language.higherKinds

/**
 * Base trait for serialize/deserialize objects.
 * @tparam O The type of plain object representation
 * @tparam B The type of binary representation
 * @tparam F Encoding/decoding effect
 */
trait Codec[F[_], O, B] {

  def encode(obj: O): F[B]

  def decode(binary: B): F[O]

}

object Codec {
  implicit def identityCodec[F[_] : Applicative, T]: Codec[F, T, T] =
    new Codec[F, T, T] {
      override def encode(obj: T): F[T] = obj.pure[F]

      override def decode(binary: T): F[T] = binary.pure[F]
    }
}
