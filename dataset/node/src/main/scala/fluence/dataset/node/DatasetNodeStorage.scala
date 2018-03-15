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
import cats.{ MonadError, ~> }
import com.typesafe.config.Config
import fluence.btree.common.merkle.MerkleRootCalculator
import fluence.btree.common.ValueRef
import fluence.btree.core.Hash
import fluence.btree.protocol.BTreeRpc
import fluence.btree.protocol.BTreeRpc.{ SearchCallback, PutCallbacks }
import fluence.btree.server.MerkleBTree
import fluence.btree.server.commands.{ SearchCommandImpl, PutCommandImpl }
import fluence.codec.Codec
import fluence.crypto.hash.CryptoHasher
import fluence.storage.KVStore
import fluence.storage.rocksdb.{ IdSeqProvider, RocksDbStore }
import monix.eval.Task
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Dataset node storage (node side).
 *
 * @param bTreeIndex            Merkle btree index.
 * @param kVStore               Blob storage for persisting encrypted values.
 * @param merkleRootCalculator Merkle root calculator (temp? will be deleted in future)
 * @param valueIdGenerator     Generator which creates surrogate id for new value when putting to dataset store.
 * @param onMRChange            Callback that will be called when merkle root change
 */
class DatasetNodeStorage private[node] (
    bTreeIndex: MerkleBTree,
    kVStore: KVStore[Task, ValueRef, Array[Byte]],
    merkleRootCalculator: MerkleRootCalculator,
    valueIdGenerator: () ⇒ ValueRef,
    onMRChange: ByteVector ⇒ Task[Unit]
) {

  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  def get(getCallbacks: SearchCallback[Task]): Task[Option[Array[Byte]]] =
    bTreeIndex.get(SearchCommandImpl(getCallbacks)).flatMap {
      case Some(reference) ⇒
        kVStore.get(reference).map(Option(_))
      case None ⇒
        Task(None)
    }

  /**
   * Initiates ''Put'' operation in remote MerkleBTree.
   *
   * @param putCallbacks   Wrapper for all callback needed for ''Put'' operation to the BTree.
   * @param encryptedValue Encrypted value.
   * @return returns old value if old value was overridden, None otherwise.
   */
  def put(
      putCallbacks: PutCallbacks[Task],
      encryptedValue: Array[Byte]
  ): Task[Option[Array[Byte]]] = {

    // todo start transaction

    for {
      // find place into index and get value reference (id of current enc. value blob)
      valRef ← bTreeIndex.put(PutCommandImpl(merkleRootCalculator, putCallbacks, valueIdGenerator))
      // fetch old value from blob kvStore
      oldVal ← kVStore.get(valRef).attempt.map(_.toOption)
      // save new blob to kvStore
      _ ← kVStore.put(valRef, encryptedValue)
      updatedMR ← bTreeIndex.getMerkleRoot
      _ ← onMRChange(ByteVector(updatedMR.bytes))
    } yield oldVal

    // todo end transaction, revert all changes if error appears

  }

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  def remove(removeCallbacks: BTreeRpc.RemoveCallback[Task]): Task[Option[Array[Byte]]] = {

    // todo start transaction

    ???

    // todo end transaction, revert all changes if error appears

  }

}

object DatasetNodeStorage {

  /**
   * Dataset node storage (node side).
   *
   * @param datasetId Some describable name of this dataset, should be unique.
   * @param cryptoHasher Hash service uses for calculating checksums.
   * @return
   */
  def apply[F[_]](
      datasetId: String,
      rocksFactory: RocksDbStore.Factory,
      config: Config,
      cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]],
      onMRChange: ByteVector ⇒ Task[Unit]
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
        onMRChange
      )
    }

  }

}
