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
package fluence.dataset.client

import cats.MonadError
import cats.syntax.flatMap._
import fluence.dataset.protocol.storage.{ DatasetStorageApi, IndexApi, ValueStorageRpc }

import scala.language.higherKinds

/**
 * Dataset storage that allows save and retrieve some value by key from remote dataset.
 *
 * @tparam F A box for returning value
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
class DatasetStorage[F[_], K, V](
  remoteIndex: IndexApi[F, K, Long],
  remoteKVStore: ValueStorageRpc[F, Long, V]
) (implicit ME: MonadError[F, Throwable]) extends DatasetStorageApi[F, K, V] {

  override def get(key: K): F[Option[V]] =
    for {
      idx <- remoteIndex.get(key)
      value ← idx.map(remoteKVStore.get)
    } yield value

  override def put(key: K, value: V): F[Option[V]] = {

    // todo start transaction

    for {
      idx <- remoteKVStore.put(value)
      oldValIdx ← remoteIndex.put(key, idx)
      oldValue ← oldValIdx.map(remoteKVStore.get)
    } yield oldValue

    // todo end transaction, revert all changes if error appears

  }

  override def remove(key: K): F[Unit] = {

    // todo start transaction

    for {
      idx ← remoteIndex.remove(key)
      removedValue <- idx.map(remoteKVStore.remove)
    } yield removedValue

    // todo end transaction, revert all changes if error appears

  }

}
