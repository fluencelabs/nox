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

import cats.Traverse
import cats.arrow.Compose

import scala.language.higherKinds

object PureCodec {

  def apply[A, B](f: FuncE[CodecError, A, B], g: FuncE[CodecError, B, A]): PureCodec[A, B] =
    BifuncE(f, g)

  def lift[A, B](f: A ⇒ B, g: B ⇒ A): PureCodec[A, B] =
    BifuncE.lift(f, g)

  def liftEither[A, B](f: A ⇒ Either[CodecError, B], g: B ⇒ Either[CodecError, A]): PureCodec[A, B] =
    BifuncE.liftEither(f, g)

  implicit def identityPureCodec[T]: PureCodec[T, T] = lift(identity, identity)

  implicit def swapPureCodec[A, B](implicit codec: PureCodec[A, B]): PureCodec[B, A] = codec.swap

  /**
   * Generates a BifuncE, traversing the input with the given BifuncE
   */
  implicit def forTraversePureCodec[G[_]: Traverse, A, B](
    implicit codec: PureCodec[A, B]
  ): PureCodec[G[A], G[B]] = {
    import FuncE.{forTraverseFuncE ⇒ funcE}
    apply(funcE(Traverse[G], codec.direct), funcE(Traverse[G], codec.inverse))
  }

  implicit object pureCodecCompose extends Compose[PureCodec] {
    override def compose[A, B, C](f: PureCodec[B, C], g: PureCodec[A, B]): PureCodec[A, C] =
      BifuncE[CodecError, A, C](f.direct compose g.direct, g.inverse compose f.inverse)
  }
}
