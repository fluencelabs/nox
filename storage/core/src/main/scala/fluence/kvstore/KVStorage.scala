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

package fluence.kvstore

import cats.{Applicative, Monad}
import fluence.kvstore.ops.Get.KVStoreGet
import fluence.kvstore.ops.Put.KVStorePut
import fluence.kvstore.ops.Traverse.KVStoreTraverse

import scala.language.higherKinds

/**
 * Top type for any key value storage.
 */
trait KVStorage

// todo finish

trait KVStoreRemove[K, V, E <: StoreError] extends KVStorage {

  def remove[F[_]](key: K)(implicit F: Monad[F]): F[Either[E, Unit]]

}

trait Snapshot[S <: KVStorage] {

  def createSnapshot[F[_]: Applicative](): F[S]

}

trait KVStoreRead[K, V, E <: StoreError] extends KVStoreGet[K, V, E] with KVStoreTraverse[K, V, E]

trait KVStoreWrite[K, V, E <: StoreError] extends KVStorePut[K, V, E] with KVStoreRemove[K, V, E]

trait ReadWriteKVStore[K, V, E <: StoreError] extends KVStoreRead[K, V, E] with KVStoreWrite[K, V, E]
