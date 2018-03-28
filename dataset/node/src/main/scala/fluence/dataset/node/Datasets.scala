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

package fluence.dataset.node

import cats.~>
import com.typesafe.config.Config
import fluence.btree.protocol.BTreeRpc
import fluence.crypto.hash.CryptoHasher
import fluence.dataset.node.DatasetNodeStorage.DatasetChanged
import fluence.dataset.protocol.DatasetStorageRpc
import fluence.kad.protocol.Key
import fluence.storage.rocksdb.RocksDbStore
import monix.eval.Task
import monix.reactive.Observable
import scodec.bits.{Bases, ByteVector}

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
  contractUpdated: (Key, DatasetChanged) ⇒ Task[Unit]
) extends DatasetStorageRpc[Task, Observable] with slogging.LazyLogging {

  private val datasets = TrieMap.empty[ByteVector, Task[DatasetNodeStorage]]

  /**
   * Checks client and node dataset versions.
   * Returns existent or create new dataset store and.
   *
   * @param datasetId     Current dataset id
   * @param clientVersion Dataset version expected to the client
   * @return dataset node storage [[DatasetNodeStorage]]
   */
  private def storage(datasetId: Array[Byte], clientVersion: Long): Task[DatasetNodeStorage] = {

    val resultStorage = for {
      key ← Key.fromBytes[Task](datasetId)
      nodeExecState ← servesDataset(key)
      _ ← nodeExecState match {
        case Some(currentNodeVersion) ⇒
          checkVersions(clientVersion, currentNodeVersion) // checking client and node consistency
        case None ⇒
          Task.raiseError(new IllegalArgumentException(s"Dataset(key=$key) is not allocated on the node"))
      }
      store ← {
        val id = ByteVector(datasetId)
        datasets.getOrElseUpdate(
          id,
          DatasetNodeStorage[Task](
            id.toBase64(Bases.Alphabets.Base64Url),
            rocksFactory,
            config,
            cryptoHasher,
            datasetChanged ⇒ contractUpdated(key, datasetChanged)
          ).map { store ⇒
            logger.info(s"For dataset=$key was successfully created storage.")
            store
          }.memoizeOnSuccess
        )
      }

    } yield store

    resultStorage.onErrorRecoverWith[DatasetNodeStorage] {
      case e ⇒
        val errMsg = s"Can't create DatasetNodeStorage for Dataset(id=${ByteVector(datasetId)}, " +
          s"version=$clientVersion) cause ${e.getMessage}"
        logger.warn(errMsg, e)
        Task.raiseError(new IllegalStateException(errMsg, e))
    }

  }

  /**
   * @param datasetId    Dataset ID
   * @param version      Dataset version expected to the client
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  override def get(
    datasetId: Array[Byte],
    version: Long,
    getCallbacks: BTreeRpc.SearchCallback[Task]
  ): Task[Option[Array[Byte]]] =
    for {
      store ← storage(datasetId, version)
      result ← store.get(getCallbacks)
    } yield result

  /**
   * @param datasetId       Dataset ID
   * @param version         Dataset version expected to the client
   * @param searchCallbacks Wrapper for all callback needed for ''Range'' operation to the BTree
   * @return returns stream of found value.
   */
  override def range(
    datasetId: Array[Byte],
    version: Long,
    searchCallbacks: BTreeRpc.SearchCallback[Task]
  ): Observable[(Array[Byte], Array[Byte])] =
    for {
      store ← Observable.fromTask(storage(datasetId, version))
      stream ← store.range(searchCallbacks)
    } yield stream

  /**
   * @param datasetId      Dataset ID
   * @param version        Dataset version expected to the client
   * @param putCallbacks   Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  override def put(
    datasetId: Array[Byte],
    version: Long,
    putCallbacks: BTreeRpc.PutCallbacks[Task],
    encryptedValue: Array[Byte]
  ): Task[Option[Array[Byte]]] =
    for {
      store ← storage(datasetId, version)
      oldValue ← store.put(version, putCallbacks, encryptedValue)
    } yield oldValue

  /**
   * @param datasetId       Dataset ID
   * @param version         Dataset version expected to the client
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  override def remove(
    datasetId: Array[Byte],
    version: Long,
    removeCallbacks: BTreeRpc.RemoveCallback[Task]
  ): Task[Option[Array[Byte]]] =
    for {
      store ← storage(datasetId, version)
      deletedValue ← store.remove(version, removeCallbacks)
    } yield deletedValue

  /**
   * Compare client and node dataset versions.
   * @param clientVersion  dataset version  expected to the client
   */
  private def checkVersions(nodeVersion: Long, clientVersion: Long): Task[Boolean] = {

    if (clientVersion > nodeVersion)
      Task.raiseError(
        new IllegalStateException(
          s"Node version '$nodeVersion' is less than client version '$clientVersion'." +
            s"Node is in inconsistent state. Try to reconnect to another node."
        )
      )
    // todo if client version < node version, send to client newest dataset state
    else if (clientVersion < nodeVersion)
      Task.raiseError(
        new NotImplementedError(
          s"Node version '$nodeVersion' is more than client version '$clientVersion'." +
            s"Node should send to client newest version and mRoot, but this functionality is not yet implemented!"
        )
      )
    else
      Task(true)
  }

  private implicit def runId[F[_]]: F ~> F = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

}
