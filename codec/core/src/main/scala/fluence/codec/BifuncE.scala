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

import cats.MonadError
import cats.arrow.Compose

import scala.language.higherKinds
import scala.language.implicitConversions

case class BifuncE[E <: Throwable, A, B](direct: FuncE[E, A, B], inverse: FuncE[E, B, A]) {
  def swap: BifuncE[E, B, A] = BifuncE(inverse, direct)

  // This is given with Compose, but IDEA fails to perform typecheck due to kind projector
  def andThen[C](other: BifuncE[E, B, C]): BifuncE[E, A, C] =
    implicitly[Compose[BifuncE[E, ?, ?]]].andThen(this, other)

  def toCodec[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, A, B] =
    Codec(direct.toKleisli.run, inverse.toKleisli.run)
}

object BifuncE extends PureCodecInstances {

  def lift[E <: Throwable, A, B](f: A ⇒ B, g: B ⇒ A): BifuncE[E, A, B] =
    BifuncE(FuncE.lift(f), FuncE.lift(g))

  def liftEither[E <: Throwable, A, B](f: A ⇒ Either[E, B], g: B ⇒ Either[E, A]): BifuncE[E, A, B] =
    BifuncE(FuncE.liftEither(f), FuncE.liftEither(g))

}
