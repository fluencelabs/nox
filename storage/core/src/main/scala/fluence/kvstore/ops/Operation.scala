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

package fluence.kvstore.ops

import cats.Monad
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.kvstore.StoreError

import scala.language.higherKinds

/**
 * Representation for kvStore lazy operations.
 *
 * @tparam V A type of returned value
 */
trait Operation[V] {

  /**
   *  A type for any storage errors
   */
  type E = StoreError

  /**
   * Runs operation using the user defined monad,
   * returns EitherT-wrapped result.
   *
   * @tparam F User defined type of monad
   */
  def run[F[_]: Monad: LiftIO]: EitherT[F, E, V]

  /**
   * Runs operation using the user defined monad,
   * returns EitherT-wrapped result.
   * The same as `run` but with converting the error for comfort use.
   *
   * @param mapErr Fn to map error type from E to EE
   * @tparam F User defined type of monad
   */
  def run[F[_]: Monad: LiftIO, EE <: Throwable](mapErr: E ⇒ EE): EitherT[F, EE, V] =
    run[F].leftMap(mapErr)

  /**
   * Runs unsafe operation, '''throw the error if it happens'''.
   * Intended to be used '''only in tests'''.
   */
  final def runUnsafe(): V =
    runF[IO].unsafeRunSync()

  /**
   * Runs operation using the user defined monad,
   * returns Either wrapped to F.
   *
   * @tparam F User defined type of monad
   */
  final def runEither[F[_]: Monad: LiftIO]: F[Either[E, V]] =
    run[F].value

  /**
   * Runs operation using the user defined MonadError,
   * lifts an error into MonadError effect.
   *
   * @tparam F User defined type of monad
   */
  final def runF[F[_]: Monad: LiftIO]: F[V] =
    runEither.map(IO.fromEither).flatMap(_.to[F])

}
