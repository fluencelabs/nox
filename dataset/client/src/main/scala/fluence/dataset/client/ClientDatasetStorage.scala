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

import fluence.btree.client.MerkleBTreeClientApi
import fluence.crypto.cipher.Crypt
import fluence.crypto.hash.CryptoHasher
import fluence.dataset.protocol.storage.{ ClientDatasetStorageApi, DatasetStorageRpc }
import monix.eval.Task

import scala.language.higherKinds

/**
 * Dataset storage that allows save and retrieve some value by key from remote dataset.
 *
 * @param bTreeIndex  An interface to dataset index.
 * @param storageRpc  Remotely-accessible interface to all dataset storage operation.
 *
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
class ClientDatasetStorage[K, V](
    bTreeIndex: MerkleBTreeClientApi[Task, K],
    storageRpc: DatasetStorageRpc[Task],
    valueCrypt: Crypt[Task, V, Array[Byte]],
    hasher: CryptoHasher[Array[Byte], Array[Byte]]
) extends ClientDatasetStorageApi[Task, K, V] {

  override def get(key: K): Task[Option[V]] = {

    for {
      getCallbacks ← bTreeIndex.initGet(key)
      serverResponse ← {
        storageRpc.get(getCallbacks)
          .doOnFinish { _ ⇒ getCallbacks.recoverState() }
      }
      resp ← decryptOption(serverResponse)
    } yield resp

  }

  override def put(key: K, value: V): Task[Option[V]] = {

    for {
      // todo crypt and hasher should be with effect, they can throw exceptions
      encValue ← valueCrypt.encrypt(value)
      encValueHash ← Task(hasher.hash(encValue))
      putCallbacks ← bTreeIndex.initPut(key, encValueHash)
      serverResponse ← {
        storageRpc
          .put(putCallbacks, encValue)
          .doOnFinish {
            // in error case we should return old value of clientState back
            case Some(e) ⇒ putCallbacks.recoverState()
            case x       ⇒ Task()
          }
      }
      resp ← decryptOption(serverResponse)
    } yield resp

  }

  override def remove(key: K): Task[Option[V]] = {

    for {
      removeCmd ← bTreeIndex.removeState(key)
      serverResponse ← storageRpc.remove(removeCmd)
      resp ← decryptOption(serverResponse)
    } yield resp

  }

  def decryptOption(response: Option[Array[Byte]]): Task[Option[V]] = {
    response match {
      case Some(r) ⇒ valueCrypt.decrypt(r).map(Option.apply)
      case None    ⇒ Task(None)
    }
  }

}
