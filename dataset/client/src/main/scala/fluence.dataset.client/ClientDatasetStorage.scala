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
import cats.syntax.functor._
import fluence.btree.client.MerkleBTreeClientApi
import fluence.crypto.cipher.Crypt
import fluence.crypto.hash.CryptoHasher
import fluence.dataset.protocol.storage.{ ClientDatasetStorageApi, DatasetStorageRpc }

import scala.language.higherKinds

/**
 * Dataset storage that allows save and retrieve some value by key from remote dataset.
 *
 * @param bTreeIndex  An interface to dataset index.
 * @param storageRpc  Remotely-accessible interface to all dataset storage operation.
 *
 * @tparam F A box for returning value
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
class ClientDatasetStorage[F[_], K, V]( // todo write integration test
    bTreeIndex: MerkleBTreeClientApi[F, K],
    storageRpc: DatasetStorageRpc[F],
    valueCrypt: Crypt[V, Array[Byte]],
    hasher: CryptoHasher[Array[Byte], Array[Byte]]
)(implicit ME: MonadError[F, Throwable]) extends ClientDatasetStorageApi[F, K, V] {

  override def get(key: K): F[Option[V]] = {

    for {
      getCmd ← bTreeIndex.getCallbacks(key)
      serverResponse ← storageRpc.get(getCmd)
    } yield serverResponse.map(valueCrypt.decrypt)

  }

  override def put(key: K, value: V): F[Option[V]] = {

    // todo start transaction

    val encryptedValue = valueCrypt.encrypt(value)
    val encryptedValueHash = hasher.hash(encryptedValue)

    for {
      putCmd ← bTreeIndex.putCallbacks(key, encryptedValueHash)
      serverResponse ← storageRpc.put(putCmd, encryptedValue)
    } yield serverResponse.map(valueCrypt.decrypt)

    // todo end transaction, revert all changes if error appears
  }

  override def remove(key: K): F[Option[V]] = {

    // todo start transaction

    for {
      removeCmd ← bTreeIndex.removeCallbacks(key)
      serverResponse ← storageRpc.remove(removeCmd)
    } yield serverResponse.map(valueCrypt.decrypt)

    // todo end transaction, revert all changes if error appears

  }

}
