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

import cats.{Monad, MonadError, Traverse}
import cats.arrow.{ArrowChoice, Compose}
import cats.data.{EitherT, Kleisli}
import cats.syntax.flatMap._
import cats.syntax.compose._

import scala.language.higherKinds
import scala.util.Try

/**
 * MonadicalEitherArrow wraps Func and Bijection with a fixed E error type.
 *
 * @tparam E Error type
 */
abstract class MonadicalEitherArrow[E <: Throwable] {

  /**
   * Alias for error type
   */
  final type Error = E

  /**
   * Func is a special kind of arrow: A => EitherT[F, E, B] for any monad F[_].
   * This signature makes the type both flexible and pure. It could be eager or lazy, synchronous or not depending on the
   * call site context.
   *
   * @tparam A Input type
   * @tparam B Successful result type
   */
  abstract class Func[A, B] {

    /**
     * Run the func on input, using the given monad.
     *
     * @param input Input
     * @tparam F All internal maps and composes are to be executed with this Monad
     */
    def runEither[F[_]: Monad](input: A): F[Either[E, B]] = apply[F](input).value

    /**
     * Run the func on input, lifting the error into MonadError effect.
     *
     * @param input Input
     * @param F All internal maps and composes, as well as errors, are to be executed with this Monad
     */
    def runF[F[_]](input: A)(implicit F: MonadError[F, Throwable]): F[B] =
      runEither(input).flatMap(F.fromEither)

    /**
     * Run the func on input, producing EitherT-wrapped result.
     *
     * @param input Input
     * @tparam F All internal maps and composes are to be executed with this Monad
     */
    def apply[F[_]: Monad](input: A): EitherT[F, E, B]

    /**
     * Converts this Func to Kleisli, using MonadError to execute upon and to lift errors into.
     *
     * @param F All internal maps and composes, as well as errors, are to be executed with this Monad
     */
    def toKleisli[F[_]](implicit F: MonadError[F, Throwable]): Kleisli[F, A, B] =
      Kleisli(input ⇒ runF[F](input))

    /**
     * Run the function, throw the error if it happens. Intended to be used only in tests.
     *
     * @param input Input
     * @return Value, or throw exception of type E
     */
    def unsafe(input: A): B = {
      import cats.instances.try_._
      runF[Try](input).get
    }
  }

  /**
   * Bijection is a transformation A => B with inverse B => A, both being partial functions which may fail with E.
   *
   * @param direct Direct transformation
   * @param inverse Inverse transformation
   * @tparam A Source type
   * @tparam B Target type
   */
  case class Bijection[A, B](direct: Func[A, B], inverse: Func[B, A]) {

    /**
     * Bijection with source and target types swapped
     */
    lazy val swap: Bijection[B, A] = Bijection(inverse, direct)

    @deprecated(
      "You should keep codec Pure until running direct or inverse on it: there's no reason to bind effect into Codec",
      "6.4.2018"
    )
    def toCodec[F[_]](implicit F: MonadError[F, Throwable]): Codec[F, A, B] =
      Codec(direct.runF[F], inverse.runF[F])
  }

  /**
   * Build a Func from an instance of Func with another error type EE.
   *
   * @param other Other Func.
   * @param convertE A way to convert other error into E.
   * @tparam EE Other Func's error.
   * @tparam A Source type
   * @tparam B Target type
   * @return Func, produced from the other func by leftMapping its result
   */
  implicit def fromOtherFunc[EE <: Throwable, A, B](
    other: MonadicalEitherArrow[EE]#Func[A, B]
  )(implicit convertE: EE ⇒ E): Func[A, B] =
    new Func[A, B] {
      override def apply[F[_]: Monad](input: A): EitherT[F, E, B] =
        other(input).leftMap(convertE)
    }

  /**
   * Provides a Func for any traversable type, given the scalar Func.
   *
   * @param f Scalar func
   * @tparam G Traversable type
   * @tparam A Source type
   * @tparam B Target type
   */
  implicit def funcForTraverse[G[_]: Traverse, A, B](implicit f: Func[A, B]): Func[G[A], G[B]] =
    new Func[G[A], G[B]] {
      override def apply[F[_]: Monad](input: G[A]): EitherT[F, E, G[B]] =
        Traverse[G].traverse[EitherT[F, E, ?], A, B](input)(f.apply[F](_))
    }

  /**
   * Bijection Summoner -- useful for making a composition of bijections.
   */
  def apply[A, B](implicit bijection: Bijection[A, B]): Bijection[A, B] = bijection

  /**
   * Lifts a pure function into Func context.
   *
   * @param f Function to lift
   */
  def liftFunc[A, B](f: A ⇒ B): Func[A, B] = new Func[A, B] {
    override def apply[F[_]: Monad](input: A): EitherT[F, E, B] =
      EitherT.rightT[F, E](input).map(f)
  }

  /**
   * Lift a pure function, returning Either, into Func context.
   *
   * @param f Function to lift
   */
  def liftFuncEither[A, B](f: A ⇒ Either[E, B]): Func[A, B] = new Func[A, B] {
    override def apply[F[_]: Monad](input: A): EitherT[F, E, B] =
      EitherT.rightT[F, E](input).subflatMap(f)
  }

  /**
   * Func that does nothing with input.
   */
  implicit def identityFunc[T]: Func[T, T] = liftFunc(identity)

  /**
   * Func should obey ArrowChoiceLaws
   */
  implicit object catsMonadicalEitherArrowChoice extends ArrowChoice[Func] {
    override def choose[A, B, C, D](f: Func[A, C])(g: Func[B, D]): Func[Either[A, B], Either[C, D]] =
      new Func[Either[A, B], Either[C, D]] {
        override def apply[F[_]: Monad](input: Either[A, B]): EitherT[F, E, Either[C, D]] =
          input.fold(
            f(_).map(Left(_)),
            g(_).map(Right(_))
          )
      }

    override def lift[A, B](f: A ⇒ B): Func[A, B] =
      liftFunc(f)

    override def first[A, B, C](fa: Func[A, B]): Func[(A, C), (B, C)] = new Func[(A, C), (B, C)] {
      override def apply[F[_]: Monad](input: (A, C)): EitherT[F, E, (B, C)] =
        fa(input._1).map(_ → input._2)
    }

    override def compose[A, B, C](f: Func[B, C], g: Func[A, B]): Func[A, C] =
      new Func[A, C] {
        override def apply[F[_]: Monad](input: A): EitherT[F, E, C] =
          g(input).flatMap(f(_))
      }
  }

  /**
   * Lifts pure direct and inverse functions into Bijection.
   *
   * @param direct Pure direct transformation.
   * @param inverse Pure inverse transformation.
   * @tparam A Source type.
   * @tparam B Target type.
   */
  def liftB[A, B](direct: A ⇒ B, inverse: B ⇒ A): Bijection[A, B] =
    Bijection(liftFunc(direct), liftFunc(inverse))

  /**
   * Lifts partial direct and inverse functions (returning errors with Either) into Bijection.
   *
   * @param direct Partial direct transformation.
   * @param inverse Partial inverse transformation.
   * @tparam A Source type.
   * @tparam B Target type.
   */
  def liftEitherB[A, B](direct: A ⇒ Either[E, B], inverse: B ⇒ Either[E, A]): Bijection[A, B] =
    Bijection(liftFuncEither(direct), liftFuncEither(inverse))

  /**
   * Bijection that does no transformation.
   */
  implicit def identityBijection[T]: Bijection[T, T] = Bijection(identityFunc, identityFunc)

  /**
   * Bijection for any traversable type.
   *
   * @param bijection Scalar bijection
   * @tparam G Traversable type
   * @tparam A Source type
   * @tparam B Target type
   */
  implicit def traversableBijection[G[_]: Traverse, A, B](implicit bijection: Bijection[A, B]): Bijection[G[A], G[B]] =
    Bijection(funcForTraverse(Traverse[G], bijection.direct), funcForTraverse(Traverse[G], bijection.inverse))

  /**
   * Gives a bijection with source and target types swapped.
   */
  implicit def swap[A, B](implicit bijection: Bijection[A, B]): Bijection[B, A] = bijection.swap

  /**
   * Bijection should obey ComposeLaws
   */
  implicit object catsMonadicalBijectionCompose extends Compose[Bijection] {
    override def compose[A, B, C](f: Bijection[B, C], g: Bijection[A, B]): Bijection[A, C] =
      Bijection(f.direct compose g.direct, g.inverse compose f.inverse)
  }
}
