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
import cats.syntax.show._
import fluence.storage.rocksdb.RocksDbStore
import cats.~>
import monix.execution.atomic.AtomicLong
import scodec.bits.{ Bases, ByteVector }

import scala.collection.concurrent.TrieMap
import scala.language.higherKinds

/**
 * Node implementation for [[DatasetStorageRpc]].
 * Caches launched [[DatasetNodeStorage]]s, routes client requests for them.
 *
 * @param config Typesafe config to use in [[DatasetNodeStorage]]
 * @param cryptoHasher Used in b-tree
 * @param servesDataset Check whether this node serves particular dataset or not
 */
// todo create unit test!
class Datasets(
    config: Config,
    rocksFactory: RocksDbStore.Factory,
    cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]],
    servesDataset: Key ⇒ Task[Option[Long]],
    contractUpdated: (Key, Long, ByteVector) ⇒ Task[Unit] // TODO: pass signature as well
) extends DatasetStorageRpc[Task] with slogging.LazyLogging {

  private val datasets = TrieMap.empty[ByteVector, Task[DatasetNodeStorage]]

  private def storage(datasetId: Array[Byte]): Task[DatasetNodeStorage] = {
    val id = ByteVector(datasetId)

    val newDataset = for {
      key ← Key.fromBytes[Task](datasetId)
      ds ← servesDataset(key).flatMap {
        case Some(currentVersion) ⇒ // TODO: ensure merkle roots matches
          val version = AtomicLong(currentVersion)

          DatasetNodeStorage[Task](
            id.toBase64(Bases.Alphabets.Base64Url),
            rocksFactory,
            config,
            cryptoHasher,
            mrHash ⇒ contractUpdated(key, version.incrementAndGet(), ByteVector(mrHash)) // TODO: there should be signature
          )

        case None ⇒
          Task.raiseError(new IllegalArgumentException(s"Dataset(key=${key.show}) is not allocated on the node"))
      }.onErrorRecoverWith[DatasetNodeStorage] {
        case e ⇒
          //todo this block is temporary convenience, will be removed when grpc will be serialize errors
          val errMsg = s"Can't create DatasetNodeStorage for datasetId=${key.show}"
          logger.warn(errMsg, e)
          Task.raiseError(new IllegalStateException(errMsg, e))
      }
    } yield {
      logger.info(s"For dataset=${key.show} was successfully created storage.")
      ds
    }

    datasets.getOrElseUpdate(id, newDataset.memoizeOnSuccess)
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

  private implicit def runId[F[_]]: F ~> F = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

}
