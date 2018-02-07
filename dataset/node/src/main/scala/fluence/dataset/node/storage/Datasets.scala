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

package fluence.dataset.node.storage

import fluence.btree.common
import fluence.btree.protocol.BTreeRpc
import fluence.crypto.hash.CryptoHasher
import fluence.dataset.protocol.storage.DatasetStorageRpc
import monix.eval.Task
import monix.execution.atomic.{ AtomicInt, AtomicLong }
import scodec.bits.{ Bases, ByteVector }

import scala.collection.concurrent.TrieMap

class Datasets(cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]]) extends DatasetStorageRpc[Task] {
  private val datasets = TrieMap.empty[ByteVector, Task[DatasetStorage]]

  private def storage(datasetId: Array[Byte]): Task[DatasetStorage] = {
    val id = ByteVector(datasetId)
    val nextId = AtomicLong(0l)

    datasets.getOrElseUpdate(
      id,
      DatasetStorage[Task](
        id.toBase64(Bases.Alphabets.Base64Url),
        cryptoHasher,
        () ⇒ nextId.getAndIncrement(), // TODO: keep last increment somewhere
        mrHash ⇒ () // TODO: store mrHash somewhere
      ).memoizeOnSuccess
    )
  }

  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param datasetId    Dataset ID
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  override def get(datasetId: Array[Byte], getCallbacks: BTreeRpc.GetCallbacks[Task]): Task[Option[Array[Byte]]] =
    storage(datasetId).flatMap(_.get(getCallbacks))

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param datasetId      Dataset ID
   * @param putCallbacks   Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  override def put(datasetId: Array[Byte], putCallbacks: BTreeRpc.PutCallbacks[Task], encryptedValue: Array[Byte]): Task[Option[Array[Byte]]] =
    storage(datasetId).flatMap(_.put(putCallbacks, encryptedValue))

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
   * @param datasetId       Dataset ID
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  override def remove(datasetId: Array[Byte], removeCallbacks: BTreeRpc.RemoveCallback[Task]): Task[Option[Array[Byte]]] =
    storage(datasetId).flatMap(_.remove(removeCallbacks))
}
