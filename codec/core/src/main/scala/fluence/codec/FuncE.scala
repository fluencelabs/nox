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

import cats.arrow.ArrowChoice
import cats.{Monad, MonadError, Traverse}
import cats.data.{EitherT, Kleisli}
import cats.syntax.flatMap._

import scala.language.higherKinds
import scala.language.implicitConversions

/**
 * FuncE is a special kind of arrow: A => EitherT[F, E, B] for any monad F[_]
 * This signature makes the type both flexible and pure. It could be eager or lazy, synchronous or not depending on the
 * call site context.
 * @tparam E Error type
 * @tparam A Input type
 * @tparam B Successful result type
 */
abstract class FuncE[E <: Throwable, A, B] {
  self ⇒

  /**
   * Run the function, using F monad.
   * @param a Input
   * @tparam F Monad
   */
  def apply[F[_]: Monad](a: A): EitherT[F, E, B]

  /**
   * Run the function, getting a String on the left side -- useful for EitherT composition.
   * @param a Input
   * @tparam F Monad
   */
  def applyS[F[_]: Monad](a: A): EitherT[F, String, B] =
    apply(a).leftMap(_.getMessage)

  /**
   * Wraps the function into concrete monad, lifting error side into MonadError effect.
   * @param F Monad
   * @tparam F MonadError instance to lift E into Throwable
   * @return
   */
  def toKleisli[F[_]](implicit F: MonadError[F, Throwable]): Kleisli[F, A, B] =
    Kleisli(apply(_).value.flatMap(F.fromEither))

  /**
   * Compose two FuncE
   * @param other FuncE to compose over this
   * @tparam EE Other's error type
   * @tparam C The new input type
   */
  def compose[EE <: E, C](other: FuncE[EE, C, A]): FuncE[E, C, B] =
    new FuncE[E, C, B] {
      override def apply[F[_]: Monad](a: C): EitherT[F, E, B] =
        other.apply[F](a).flatMap(self.apply[F])
    }

  /**
   * Maps the error on the left side
   */
  def leftMap[EE <: Throwable](f: E ⇒ EE): FuncE[EE, A, B] =
    new FuncE[EE, A, B] {
      override def apply[F[_]: Monad](a: A): EitherT[F, EE, B] =
        self.apply(a).leftMap(f)
    }

}

object FuncE {

  /**
   * Lift a pure function into FuncE.
   * @param f The function to lift
   * @tparam A Input
   * @tparam E Error type
   * @tparam B Output
   */
  implicit def lift[E <: Throwable, A, B](f: A ⇒ B): FuncE[E, A, B] =
    new FuncE[E, A, B] {
      override def apply[F[_]: Monad](a: A): EitherT[F, E, B] = EitherT.pure(f(a))
    }

  /**
   * Lift a pure function that uses Either to return errors.
   * @param f The function to lift
   * @tparam A Input
   * @tparam E Error type
   * @tparam B Output
   */
  implicit def liftEither[E <: Throwable, A, B](f: A ⇒ Either[E, B]): FuncE[E, A, B] =
    new FuncE[E, A, B] {
      override def apply[F[_]: Monad](a: A): EitherT[F, E, B] = EitherT.fromEither(f(a))
    }

  /**
   * FuncE that does nothing.
   * @tparam E Error type
   * @tparam T Input and output type
   */
  implicit def identityFuncE[E <: Throwable, T]: FuncE[E, T, T] = lift(identity)

  /**
   * Converts a FuncE into Kleisli, binding a concrete MonadError into it.
   */
  implicit def toKleisli[F[_], E <: Throwable, A, B](
    funcE: FuncE[E, A, B]
  )(implicit F: MonadError[F, Throwable]): Kleisli[F, A, B] =
    funcE.toKleisli

  /**
   * Generates a FuncE, traversing the input with the given FuncE
   */
  implicit def forTraverseFuncE[G[_]: Traverse, E <: Throwable, A, B](
    implicit funcE: FuncE[E, A, B]
  ): FuncE[E, G[A], G[B]] =
    new FuncE[E, G[A], G[B]] {

      /**
       * Run the function, using F monad.
       *
       * @param a Input
       * @tparam F Monad
       */
      override def apply[F[_]: Monad](a: G[A]): EitherT[F, E, G[B]] =
        Traverse[G].traverse[EitherT[F, E, ?], A, B](a)(funcE.apply)
    }

  /**
   * FuncE obeys arrow choice laws with any fixed E
   * @tparam E Error type
   */
  implicit def catsArrowChoice[E <: Throwable]: ArrowChoice[FuncE[E, ?, ?]] = {
    type G[A, B] = FuncE[E, A, B]
    new ArrowChoice[G] {
      override def choose[A, B, C, D](f: G[A, C])(g: G[B, D]): G[Either[A, B], Either[C, D]] =
        new FuncE[E, Either[A, B], Either[C, D]] {
          override def apply[F[_]: Monad](a: Either[A, B]): EitherT[F, E, Either[C, D]] =
            a.fold[EitherT[F, E, Either[C, D]]](
              l ⇒ f(l).map(Left(_)),
              r ⇒ g(r).map(Right(_))
            )
        }

      override def lift[A, B](f: A ⇒ B): G[A, B] =
        FuncE.lift(f)

      override def first[A, B, C](fa: G[A, B]): G[(A, C), (B, C)] =
        new FuncE[E, (A, C), (B, C)] {
          override def apply[F[_]: Monad](a: (A, C)): EitherT[F, E, (B, C)] = fa(a._1).map(_ -> a._2)
        }

      override def compose[A, B, C](f: G[B, C], g: G[A, B]): G[A, C] =
        f.compose(g)
    }
  }
}
