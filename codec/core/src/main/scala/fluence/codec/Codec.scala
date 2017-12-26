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

package fluence.codec

import cats.data.Kleisli
import cats.{ Applicative, Traverse }
import cats.syntax.applicative._

import scala.language.{ higherKinds, implicitConversions }

/**
 * Base trait for serialize/deserialize objects.
 *
 * @tparam O The type of plain object representation
 * @tparam B The type of binary representation
 * @tparam F Encoding/decoding effect
 */
trait Codec[F[_], O, B] {

  def encode(obj: O): F[B]

  def decode(binary: B): F[O]

  val direct: Kleisli[F, O, B] = Kleisli(encode)

  val inverse: Kleisli[F, B, O] = Kleisli(decode)

}

object Codec {
  implicit def identityCodec[F[_] : Applicative, T]: Codec[F, T, T] =
    new Codec[F, T, T] {
      override def encode(obj: T): F[T] = obj.pure[F]

      override def decode(binary: T): F[T] = binary.pure[F]
    }

  implicit def traverseCodec[F[_] : Applicative, G[_] : Traverse, O, B](implicit codec: Codec[F, O, B]): Codec[F, G[O], G[B]] =
    new Codec[F, G[O], G[B]] {
      override def encode(obj: G[O]): F[G[B]] =
        Traverse[G].traverse[F, O, B](obj)(codec.encode)

      override def decode(binary: G[B]): F[G[O]] =
        Traverse[G].traverse[F, B, O](binary)(codec.decode)
    }

  def apply[F[_], O, B](implicit codec: Codec[F, O, B]): Codec[F, O, B] = codec

  /**
   * Constructs a Codec from pure encode/decode functions and an Applicative
   *
   * @param encodeFn Encode function that never fail
   * @param decodeFn Decode function that never fail
   * @tparam F Applicative effect
   * @tparam O Raw type
   * @tparam B Encoded type
   * @return New codec for O and B
   */
  def pure[F[_] : Applicative, O, B](encodeFn: O ⇒ B, decodeFn: B ⇒ O): Codec[F, O, B] = new Codec[F, O, B] {
    override def encode(obj: O): F[B] = encodeFn(obj).pure[F]

    override def decode(binary: B): F[O] = decodeFn(binary).pure[F]
  }
}
