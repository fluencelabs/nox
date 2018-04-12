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

import cats.{~>, Applicative, Monad, MonadError}

import scala.language.higherKinds

trait KVStorage

trait KVStoreGet[K, V, E <: StoreError] extends KVStorage {

  def get[F[_]](key: K)(implicit F: Monad[F]): F[Either[E, Option[V]]]

}

trait KVStoreTraverse[K, V, E <: StoreError] extends KVStorage {

  def traverse[FS[_]]()(implicit FS: MonadError[FS, E], liftFS: Iterator ~> FS): FS[(K, V)]

}

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
