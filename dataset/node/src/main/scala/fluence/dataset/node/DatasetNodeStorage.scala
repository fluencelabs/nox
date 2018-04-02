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

import java.nio.ByteBuffer

import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{~>, MonadError}
import com.typesafe.config.Config
import fluence.btree.common.ValueRef
import fluence.btree.common.merkle.MerkleRootCalculator
import fluence.btree.core.Hash
import fluence.btree.protocol.BTreeRpc
import fluence.btree.protocol.BTreeRpc.{PutCallbacks, SearchCallback}
import fluence.btree.server.MerkleBTree
import fluence.btree.server.commands.{PutCommandImpl, SearchCommandImpl}
import fluence.codec.Codec
import fluence.crypto.hash.CryptoHasher
import fluence.crypto.signature.Signature
import fluence.dataset.node.DatasetNodeStorage.DatasetChanged
import fluence.storage.KVStore
import fluence.storage.rocksdb.{IdSeqProvider, RocksDbStore}
import monix.eval.Task
import monix.reactive.Observable
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Dataset node storage (node side).
 *
 * @param bTreeIndex            Merkle btree index.
 * @param kVStore               Blob storage for persisting encrypted values.
 * @param merkleRootCalculator Merkle root calculator (temp? will be deleted in future)
 * @param valueIdGenerator     Generator which creates surrogate id for new value when putting to dataset store.
 * @param onDatasetChange      Callback that will be called when dataset change
 */
class DatasetNodeStorage private[node] (
  bTreeIndex: MerkleBTree,
  kVStore: KVStore[Task, ValueRef, Array[Byte]],
  merkleRootCalculator: MerkleRootCalculator,
  valueIdGenerator: () ⇒ ValueRef,
  onDatasetChange: DatasetChanged ⇒ Task[Unit]
) {

  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param searchCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  def get(searchCallbacks: SearchCallback[Task]): Task[Option[Array[Byte]]] =
    bTreeIndex.get(SearchCommandImpl(searchCallbacks)).flatMap {
      case Some(reference) ⇒
        kVStore.get(reference).map(Option(_))
      case None ⇒
        Task(None)
    }

  /**
   * Initiates ''Range'' operation in remote MerkleBTree.
   *
   * @param searchCallbacks Wrapper for all callback needed for searching start key into the BTree
   * @return returns found key-value pairs as stream
   */
  def range(searchCallbacks: SearchCallback[Task]): Observable[(Array[Byte], Array[Byte])] =
    bTreeIndex.range(SearchCommandImpl(searchCallbacks)).mapTask {
      case (key, valRef) ⇒
        kVStore.get(valRef).map(value ⇒ key.bytes → value)
    }

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param version      Dataset version expected to the client
   * @param putCallbacks Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  def put(
    version: Long,
    putCallbacks: PutCallbacks[Task],
    encryptedValue: Array[Byte]
  ): Task[Option[Array[Byte]]] = {

    // todo start transaction

    for {
      putCmd ← Task(PutCommandImpl(merkleRootCalculator, putCallbacks, valueIdGenerator))
      // find place into index and get value reference (id of current enc. value blob)
      valRef ← bTreeIndex.put(putCmd)
      // fetch old value from blob kvStore
      oldVal ← kVStore.get(valRef).attempt.map(_.toOption)
      // save new blob to kvStore
      _ ← kVStore.put(valRef, encryptedValue)
      updatedMR ← bTreeIndex.getMerkleRoot
      signedState ← putCmd.getClientStateSignature
      // increment expected client version by one, because dataset change state
      _ ← onDatasetChange(DatasetChanged(ByteVector(updatedMR.bytes), version + 1, Signature(signedState)))
    } yield oldVal

    // todo end transaction, revert all changes if error appears

  }

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
   * @param version      Dataset version expected to the client
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  def remove(
    version: Long,
    removeCallbacks: BTreeRpc.RemoveCallback[Task]
  ): Task[Option[Array[Byte]]] = {

    // todo start transaction

    ???

    // todo end transaction, revert all changes if error appears

  }

}

object DatasetNodeStorage {

  case class DatasetChanged(newMRoot: ByteVector, newVersion: Long, clientSignature: Signature)

  /**
   * Dataset node storage (node side).
   *
   * @param datasetId Some describable name of this dataset, should be unique.
   * @param rocksFactory RocksDb factory for getting registered instance of RocksDb
   * @param config  Global typeSafe config for node
   * @param cryptoHasher Hash service uses for calculating checksums.
   * @param onDatasetChange Callback that will be called when dataset change
   */
  def apply[F[_]](
    datasetId: String,
    rocksFactory: RocksDbStore.Factory,
    config: Config,
    cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]],
    onDatasetChange: DatasetChanged ⇒ Task[Unit]
  )(implicit F: MonadError[F, Throwable], runTask: Task ~> F): F[DatasetNodeStorage] = {
    import Codec.identityCodec

    // todo create direct and faster codec for Long
    implicit val valRef2bytesCodec: Codec[Task, Array[Byte], ValueRef] = Codec.pure(
      ByteBuffer.wrap(_).getLong(),
      ByteBuffer.allocate(java.lang.Long.BYTES).putLong(_).array()
    )

    val wrappedHasher = cryptoHasher.map(Hash(_))

    for {
      rocksDb ← rocksFactory(s"$datasetId/blob_data", config)
      idSeqProvider ← IdSeqProvider.longSeqProvider(rocksDb)
      btreeIdx ← MerkleBTree(s"$datasetId/btree_idx", rocksFactory, wrappedHasher, config)
    } yield {
      new DatasetNodeStorage(
        btreeIdx,
        KVStore.transform(rocksDb),
        MerkleRootCalculator(wrappedHasher),
        idSeqProvider,
        onDatasetChange
      )
    }

  }

}
