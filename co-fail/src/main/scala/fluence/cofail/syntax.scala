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

package fluence.cofail

import cats.Monad
import cats.syntax.flatMap._
import cats.data.EitherT
import shapeless._

import scala.language.higherKinds

object syntax {

  sealed trait NotA[A]

  sealed trait LeftMerger[A, B] {
    type R

    def lift1(v: A): R
    def lift2(v: B): R
  }
  type LeftMergerAux[A, B, R0] = LeftMerger[A, B]{type R = R0}

  implicit def notA[A]: NotA[A] = ???
  implicit def notNotA[A](implicit a: A): NotA[A] = ???

  // TODO: it must not make a coproduct from A <: AA or A >: AA, it should merge them into supertype instead
  implicit class EitherTSyntax[F[_]: Monad, A, B](self: EitherT[F, A, B])(implicit notCoproduct: NotA[A <:< Coproduct]) {
    def flatMap[AA, D](f: B ⇒ EitherT[F, AA, D])(implicit
                                                 evLeftNotMatch: AA =:!= A,
                                                 notSubEv: NotA[A <:< AA],
                                                 notSubEvInv: NotA[AA <:< A]): EitherT[F, AA :+: A :+: CNil, D] =
      EitherT[F, AA :+: A :+: CNil, D](self.value.flatMap {
        case Right(r) ⇒
          f(r).leftMap(Coproduct[AA :+: A :+: CNil](_)).value
        case Left(l) ⇒
          Monad[F].pure(Left(Coproduct[AA :+: A :+: CNil](l)))
      })

    // if A in AA, lift it
    // if A not in AA, extend A :+: AA
    def flatMap[AA <: Coproduct, D](f: B => EitherT[F, AA, D]): EitherT[F, AA, B] = ???
  }

}
