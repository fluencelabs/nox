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

package fluence.node

import cats.MonadError
import shapeless._

import scala.util.control.NoStackTrace
import scala.language.higherKinds
import scala.reflect.ClassTag

case class CoFail[T <: Coproduct](cause: T) extends NoStackTrace

object CoFail {
  // Convert generic MonadError for Throwable into CoFail of particular type
  def fromThrowableME[F[_], T <: Coproduct : ClassTag](ME: MonadError[F, Throwable]): MonadError[F, T] =
    new MonadError[F, T] {
      override def flatMap[A, B](fa: F[A])(f: A ⇒ F[B]): F[B] = ME.flatMap(fa)(f)

      override def tailRecM[A, B](a: A)(f: A ⇒ F[Either[A, B]]): F[B] = ME.tailRecM(a)(f)

      override def raiseError[A](e: T): F[A] = ME.raiseError(CoFail(e))

      override def handleErrorWith[A](fa: F[A])(f: T ⇒ F[A]): F[A] = ME.handleErrorWith(fa){
        case cf: T ⇒ f(cf)
        case t     ⇒ ME.raiseError(t)
      }

      override def pure[A](x: A): F[A] = ME.pure(x)
    }

  // Pick subset of CoFail errors into a new narrowed MonadError
  implicit def narrowCoFail[F[_], T <: Coproduct, TT <: Coproduct](implicit
    ME: MonadError[F, T],
    basis: Lazy[ops.coproduct.Basis[T, TT]],
    typesNeqEvidence: T =:!= TT): MonadError[F, TT] =
    new MonadError[F, TT] {
      override def flatMap[A, B](fa: F[A])(f: A ⇒ F[B]): F[B] = ME.flatMap(fa)(f)

      override def tailRecM[A, B](a: A)(f: A ⇒ F[Either[A, B]]): F[B] = ME.tailRecM(a)(f)

      override def raiseError[A](e: TT): F[A] =
        ME.raiseError(basis.value.inverse(Right(e)))

      override def handleErrorWith[A](fa: F[A])(f: TT ⇒ F[A]): F[A] = ME.handleErrorWith(fa){
        cf ⇒
          basis.value.apply(cf) match {
            case Left(_) ⇒
              ME.raiseError(cf)

            case Right(r) ⇒
              f(r)
          }
      }

      override def pure[A](x: A): F[A] = ME.pure(x)
    }

  // Pick a single failure from MonadError to a new MonadError
  implicit def pickCoFail[F[_], T <: Coproduct, TT](implicit
    ME: MonadError[F, T],
    select: Lazy[ops.coproduct.Selector[T, TT]],
    inject: Lazy[ops.coproduct.Inject[T, TT]]
  ): MonadError[F, TT] =
    new MonadError[F, TT] {

      override def flatMap[A, B](fa: F[A])(f: A ⇒ F[B]): F[B] = ME.flatMap(fa)(f)

      override def tailRecM[A, B](a: A)(f: A ⇒ F[Either[A, B]]): F[B] = ME.tailRecM(a)(f)

      override def raiseError[A](e: TT): F[A] = ME.raiseError(inject.value(e))

      override def handleErrorWith[A](fa: F[A])(f: TT ⇒ F[A]): F[A] = ME.handleErrorWith(fa){
        cf ⇒
          select.value(cf).fold(ME.raiseError[A](cf))(f(_))
      }

      override def pure[A](x: A): F[A] = ME.pure(x)
    }

  class CheckCompiles[F[_]](ME: MonadError[F, Throwable]) {

    object Implicits {
      implicit val TME: MonadError[F, Throwable] = ME

      implicit val F: MonadError[F, A :+: B :+: NoSuchElementException :+: CNil] =
        CoFail.fromThrowableME(TME)
    }

    case class A(a: String)
    case class B(b: String)

    def toCoFail: MonadError[F, A :+: B :+: CNil] = {
      import Implicits.F

      narrow()
    }

    def narrow()(implicit F: MonadError[F, A :+: B :+: NoSuchElementException :+: CNil]): MonadError[F, A :+: B :+: CNil] =
      {
        import CoFail._
        implicitly[MonadError[F, A :+: B :+: CNil]]
      }

    def pick()(implicit F: MonadError[F, A :+: B :+: NoSuchElementException :+: CNil]): MonadError[F, A] =
      {
        import CoFail._
        implicitly[MonadError[F, A]]
      }

  }
}
