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

import cats.syntax.flatMap._
import cats.{Monad, MonadError}
import cats.data.EitherT
import fluence.kvstore.{KVStorage, StoreError}

import scala.language.higherKinds

/**
 * Lazy representation for getting value by specified key.
 *
 * @tparam K A type of search key
 * @tparam V A type of value
 * @tparam E A type for any storage errors
 */
trait Get[K, V, E <: StoreError] {

  /**
   * Runs the getting value by specified key, using the user defined monad,
   * returns EitherT-wrapped result.
   *
   * @tparam F User defined type of monad
   */
  def run[F[_]: Monad]: EitherT[F, E, Option[V]]

  /**
   * Runs the getting value by specified key, using the user defined monad,
   * returns Either wrapped to F.
   *
   * @tparam F User defined type of monad
   */
  def runEither[F[_]: Monad]: F[Either[E, Option[V]]] =
    run[F].value

  /**
   * Runs the getting value by specified key, using the user defined MonadError
   * lifts an error into MonadError effect.

   * @tparam F User defined type of monad
   */
  def runF[F[_]: Monad](implicit F: MonadError[F, E]): F[Option[V]] =
    runEither.flatMap(F.fromEither)

  /**
   * Runs the getting value by specified key, '''throw the error if it happens'''.
   * Intended to be used '''only in tests'''.
   */
  def runUnsafe(): Option[V]

}

object Get {

  /**
   * Contract for obtaining values by key. In other words ''mixin'' with ''get'' functionality.
   *
   * @tparam K A type of search key
   * @tparam V A type of value
   * @tparam E A type for any storage errors
   */
  trait KVStoreGet[K, V, E <: StoreError] extends KVStorage {

    /**
     * Returns lazy ''get'' representation (see [[Get]])
     *
     * @param key Search key
     */
    def get(key: K): Get[K, V, StoreError]

  }

}
