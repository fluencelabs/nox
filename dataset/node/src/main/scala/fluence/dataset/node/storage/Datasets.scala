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

import com.typesafe.config.Config
import fluence.btree.protocol.BTreeRpc
import fluence.crypto.hash.CryptoHasher
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.kad.protocol.Key
import monix.eval.Task
import monix.execution.atomic.AtomicLong
import scodec.bits.{ Bases, ByteVector }

import scala.collection.concurrent.TrieMap

/**
 * Node implementation for [[DatasetStorageRpc]].
 * Caches launched [[DatasetStorage]]s, routes client requests for them.
 *
 * @param config Typesafe config to use in [[DatasetStorage]]
 * @param cryptoHasher Used in b-tree
 * @param servesDataset Check whether this node serves particular dataset or not
 */
class Datasets(
    config: Config,
    cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]],
    servesDataset: Key ⇒ Task[Option[Long]],
    contractUpdated: (Key, Long, ByteVector) ⇒ Task[Unit] // TODO: pass signature as well
) extends DatasetStorageRpc[Task] {

  private val datasets = TrieMap.empty[ByteVector, Task[DatasetStorage]]

  private def storage(datasetId: Array[Byte]): Task[DatasetStorage] = {
    val id = ByteVector(datasetId)

    datasets.getOrElseUpdate(
      id,

      for {
        k ← Key.fromBytes[Task](datasetId)
        ds ← servesDataset(k).flatMap {
          case Some(currentVersion) ⇒ // TODO: ensure merkle roots matches
            val version = AtomicLong(currentVersion)
            val nextId = AtomicLong(0l)

            DatasetStorage[Task](
              id.toBase64(Bases.Alphabets.Base64Url),
              config,
              cryptoHasher,
              () ⇒ nextId.getAndIncrement(), // TODO: keep last increment somewhere
              mrHash ⇒ contractUpdated(k, version.incrementAndGet(), ByteVector(mrHash)) // TODO: there should be signature
            ).memoizeOnSuccess

          case None ⇒
            Task.raiseError(new IllegalArgumentException("Dataset is not allocated on the node"))
        }
      } yield ds
    )
  }

  /**
   * @param datasetId    Dataset ID
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  override def get(datasetId: Array[Byte], getCallbacks: BTreeRpc.GetCallbacks[Task]): Task[Option[Array[Byte]]] =
    storage(datasetId).flatMap(_.get(getCallbacks))

  /**
   * @param datasetId      Dataset ID
   * @param putCallbacks   Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  override def put(datasetId: Array[Byte], putCallbacks: BTreeRpc.PutCallbacks[Task], encryptedValue: Array[Byte]): Task[Option[Array[Byte]]] =
    storage(datasetId).flatMap(_.put(putCallbacks, encryptedValue))

  /**
   * @param datasetId       Dataset ID
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  override def remove(datasetId: Array[Byte], removeCallbacks: BTreeRpc.RemoveCallback[Task]): Task[Option[Array[Byte]]] =
    storage(datasetId).flatMap(_.remove(removeCallbacks))
}
