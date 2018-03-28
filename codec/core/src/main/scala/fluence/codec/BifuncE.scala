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
import scala.language.implicitConversions

case class BifuncE[E <: Throwable, A, B](direct: FuncE[E, A, B], inverse: FuncE[E, B, A]) {
  def swap: BifuncE[E, B, A] = BifuncE(inverse, direct)
}

object BifuncE {

  def lift[E <: Throwable, A, B](f: A ⇒ B, g: B ⇒ A): BifuncE[E, A, B] =
    BifuncE(FuncE.lift(f), FuncE.lift(g))

  def liftEither[E <: Throwable, A, B](f: A ⇒ Either[E, B], g: B ⇒ Either[E, A]): BifuncE[E, A, B] =
    BifuncE(FuncE.liftEither(f), FuncE.liftEither(g))

  implicit def identityBifuncE[E <: Throwable, T]: BifuncE[E, T, T] = lift(identity, identity)

  implicit def swapBifuncE[E <: Throwable, A, B](implicit bifuncE: BifuncE[E, A, B]): BifuncE[E, B, A] = bifuncE.swap

  /**
   * Generates a BifuncE, traversing the input with the given BifuncE
   */
  implicit def forTraverseBifuncE[G[_]: Traverse, E <: Throwable, A, B](
    implicit bifuncE: BifuncE[E, A, B]
  ): BifuncE[E, G[A], G[B]] = {
    import FuncE.{forTraverseFuncE ⇒ funcE}
    BifuncE[E, G[A], G[B]](funcE(Traverse[G], bifuncE.direct), funcE(Traverse[G], bifuncE.inverse))
  }

  implicit def bifuncECompose[E <: Throwable]: Compose[BifuncE[E, ?, ?]] = new Compose[BifuncE[E, ?, ?]] {
    type BE[A, B] = BifuncE[E, A, B]

    override def compose[A, B, C](f: BE[B, C], g: BE[A, B]): BE[A, C] =
      BifuncE(f.direct compose g.direct, g.inverse compose f.inverse)
  }

}
