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
import cats.{ Applicative, FlatMap, Traverse }
import cats.syntax.applicative._

import scala.language.{ higherKinds, implicitConversions }

/**
 * Base trait for serialize/deserialize objects.
 *
 * @tparam A The type of plain object representation
 * @tparam B The type of binary representation
 * @tparam F Encoding/decoding effect
 */
final case class Codec[F[_], A, B](encode: A ⇒ F[B], decode: B ⇒ F[A]) {
  self ⇒

  val direct: Kleisli[F, A, B] = Kleisli(encode)

  val inverse: Kleisli[F, B, A] = Kleisli(decode)

  def andThen[C](other: Codec[F, B, C])(implicit F: FlatMap[F]): Codec[F, A, C] =
    Codec((self.direct andThen other.direct).run, (other.inverse andThen self.inverse).run)
}

object Codec {
  implicit def identityCodec[F[_] : Applicative, T]: Codec[F, T, T] =
    Codec(_.pure[F], _.pure[F])

  implicit def traverseCodec[F[_] : Applicative, G[_] : Traverse, O, B](implicit codec: Codec[F, O, B]): Codec[F, G[O], G[B]] =
    Codec[F, G[O], G[B]](Traverse[G].traverse[F, O, B](_)(codec.encode), Traverse[G].traverse[F, B, O](_)(codec.decode))

  def codec[F[_], O, B](implicit codec: Codec[F, O, B]): Codec[F, O, B] = codec

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
  def pure[F[_] : Applicative, O, B](encodeFn: O ⇒ B, decodeFn: B ⇒ O): Codec[F, O, B] =
    Codec(encodeFn(_).pure[F], decodeFn(_).pure[F])
}
