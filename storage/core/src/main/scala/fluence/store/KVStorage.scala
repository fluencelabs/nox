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

package fluence.store

import cats.data.EitherT
import cats.syntax.flatMap._
import cats.{~>, Applicative, Monad, MonadError}

import scala.language.higherKinds

/**
 * Top type for any key value storage.
 */
trait KVStorage

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
   */
  def get: Get[K, V, StoreError]

}

/**
 * Lazy representation for getting value by specified key.
 *
 * @tparam K A type of search key
 * @tparam V A type of value
 * @tparam E A type for any storage errors
 */
trait Get[K, V, E <: StoreError] {

  /**
   * Runs the getting value by specified key, using the given monad,
   * returns EitherT-wrapped result.
   *
   * @param key Search key
   * @tparam F User defined type of monad
   */
  def run[F[_]: Monad](key: K): EitherT[F, E, Option[V]]

  /**
   * Runs the getting value by specified key, using the user defined monad,
   * returns Either wrapped to F.
   *
   * @param key Search key
   * @tparam F User defined type of monad
   */
  def runEither[F[_]: Monad](key: K): F[Either[E, Option[V]]] =
    run[F](key).value

  /**
   * Runs the getting value by specified key, using the user defined MonadError
   * lifts an error into MonadError effect.
   *
   * @param key Search key
   * @tparam F User defined type of monad
   */
  def runF[F[_]: Monad](key: K)(implicit F: MonadError[F, StoreError]): F[Option[V]] =
    runEither(key).flatMap(F.fromEither)

  /**
   * Runs the getting value by specified key, '''throw the error if it happens'''.
   * Intended to be used '''only in tests'''.
   *
   * @param key Search key
   */
  def runUnsafe(key: K): Option[V]

}

/**
 * Contract for traversing all key-value pairs.
 * In other words ''mixin'' with ''traverse'' functionality.
 *
 * @tparam K A type of search key
 * @tparam V A type of value
 * @tparam E A type for any storage errors
 */
trait KVStoreTraverse[K, V, E <: StoreError] extends KVStorage {

  /**
   * Returns lazy ''traverse'' representation (see [[Traverse]])
   */
  def traverse: Traverse[K, V, E]

}

/**
 * Lazy representation for traversing all values.
 *
 * @tparam K A type of search key
 * @tparam V A type of value
 * @tparam E A type for any storage errors
 */
trait Traverse[K, V, E <: StoreError] {

  /**
   * Returns FS stream of all pairs in current key-value store.
   *
   * @param FS MonadError type class for user defined type FS
   * @param liftIterator Creates FS stream from [[Iterator]]
   *
   * @tparam FS User defined type of monadError
   */
  def run[FS[_]: Monad](implicit FS: MonadError[FS, E], liftIterator: Iterator ~> FS): FS[(K, V)]

  /**
   * Returns [[Iterator]] with all key-value pairs for current KVStore,
   * '''throw the error if it happens'''. Intended to be used '''only in tests'''.
   */
  def runUnsafe: Iterator[(K, V)]

}

// todo finish

trait KVStorePut[K, V, E <: StoreError] extends KVStorage {

  def put[F[_]](key: K, value: V)(implicit F: Monad[F]): F[Either[E, Unit]]

}

trait KVStoreRemove[K, V, E <: StoreError] extends KVStorage {

  def remove[F[_]](key: K)(implicit F: Monad[F]): F[Either[E, Unit]]

}

trait Snapshot[S <: KVStorage] {

  def createSnapshot[F[_]: Applicative](): F[S]

}

trait KVStoreRead[K, V, E <: StoreError] extends KVStoreGet[K, V, E] with KVStoreTraverse[K, V, E]

trait KVStoreWrite[K, V, E <: StoreError] extends KVStorePut[K, V, E] with KVStoreRemove[K, V, E]

trait ReadWriteKVStore[K, V, E <: StoreError] extends KVStoreRead[K, V, E] with KVStoreWrite[K, V, E]
