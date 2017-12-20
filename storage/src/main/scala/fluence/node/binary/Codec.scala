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
