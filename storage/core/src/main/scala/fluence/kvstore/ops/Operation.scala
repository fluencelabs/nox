package fluence.kvstore.ops

import cats.syntax.flatMap._
import cats.{Monad, MonadError}
import cats.data.EitherT
import fluence.kvstore.StoreError

import scala.language.higherKinds

/**
 * Representation for kvStore lazy operations.
 *
 * @tparam K A type of search key
 * @tparam V A type of value
 * @tparam E A type for any storage errors
 */
trait Operation[K, V, E <: StoreError] {

  /**
   * Runs operation using the user defined monad,
   * returns EitherT-wrapped result.
   *
   * @tparam F User defined type of monad
   */
  def run[F[_]: Monad]: EitherT[F, E, V]

  /**
   * Runs unsafe operation, '''throw the error if it happens'''.
   * Intended to be used '''only in tests'''.
   */
  def runUnsafe(): V

  /**
   * Runs operation using the user defined monad,
   * returns Either wrapped to F.
   *
   * @tparam F User defined type of monad
   */
  def runEither[F[_]: Monad]: F[Either[E, V]] =
    run[F].value

  /**
   * Runs operation using the user defined MonadError,
   * lifts an error into MonadError effect.
   *
   * @tparam F User defined type of monad
   */
  def runF[F[_]: Monad](implicit F: MonadError[F, E]): F[V] =
    runEither.flatMap(F.fromEither)

}
