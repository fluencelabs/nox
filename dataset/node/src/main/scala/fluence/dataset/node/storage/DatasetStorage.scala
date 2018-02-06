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

import java.nio.ByteBuffer

import fluence.btree.common.{ Bytes, ValueRef }
import fluence.btree.common.merkle.MerkleRootCalculator
import fluence.btree.protocol.BTreeRpc
import fluence.btree.protocol.BTreeRpc.{ GetCallbacks, PutCallbacks }
import fluence.btree.server.MerkleBTree
import fluence.btree.server.commands.{ GetCommandImpl, PutCommandImpl }
import fluence.codec.Codec
import fluence.crypto.hash.CryptoHasher
import fluence.dataset.protocol.storage.DatasetStorageRpc
import fluence.storage.KVStore
import fluence.storage.rocksdb.RocksDbStore
import monix.eval.Task

import scala.language.higherKinds
import scala.util.Try

/**
 * Dataset node storage (node side). Base implementation of [[DatasetStorageRpc]]
 *
 * @param bTreeIndex            Merkle btree index.
 * @param kVStore               Blob storage for persisting encrypted values.
 * @param merkleRootCalculator Merkle root calculator (temp? will be deleted in future)
 * @param refProvider           Next value id generator
 * @param onMRChange      Callback that will be called when merkle root change
 */
class DatasetStorage private (
    bTreeIndex: MerkleBTree,
    kVStore: KVStore[Task, ValueRef, Array[Byte]],
    merkleRootCalculator: MerkleRootCalculator,
    refProvider: () ⇒ ValueRef,
    onMRChange: Bytes ⇒ Unit
) extends DatasetStorageRpc[Task] {

  /**
   * Initiates ''Get'' operation in remote MerkleBTree.
   *
   * @param getCallbacks Wrapper for all callback needed for ''Get'' operation to the BTree
   * @return returns found value, None if nothing was found.
   */
  override def get(datasetId: Array[Byte], getCallbacks: GetCallbacks[Task]): Task[Option[Array[Byte]]] =
    bTreeIndex.get(GetCommandImpl(getCallbacks))
      .flatMap {
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
  override def put(
    datasetId: Array[Byte],
    putCallbacks: PutCallbacks[Task],
    encryptedValue: Array[Byte]
  ): Task[Option[Array[Byte]]] = {

    // todo start transaction

    for {
      // find place into index and get value reference (id of current enc. value blob)
      valRef ← bTreeIndex.put(PutCommandImpl(merkleRootCalculator, putCallbacks, refProvider))
      // fetch old value from blob kvStore
      oldVal ← kVStore.get(valRef).attempt.map(_.toOption)
      // save new blob to kvStore
      _ ← kVStore.put(valRef, encryptedValue)
      _ ← bTreeIndex.getMerkleRoot.map(onMRChange)
    } yield oldVal

    // todo end transaction, revert all changes if error appears

  }

  /**
   * Initiates ''Remove'' operation in remote MerkleBTree.
   *
   * @param removeCallbacks Wrapper for all callback needed for ''Remove'' operation to the BTree.
   * @return returns old value that was deleted, None if nothing was deleted.
   */
  override def remove(datasetId: Array[Byte], removeCallbacks: BTreeRpc.RemoveCallback[Task]): Task[Option[Array[Byte]]] = {

    // todo start transaction

    ???

    // todo end transaction, revert all changes if error appears

  }

}

object DatasetStorage {

  /**
   * Dataset node storage (node side).
   *
   * @param datasetId Some describable name of this dataset, should be unique.
   * @param cryptoHasher Hash service uses for calculating checksums.
   * @param refProvider  A function for getting next value reference
   * @return
   */
  def apply(
    datasetId: String,
    cryptoHasher: CryptoHasher[Array[Byte], Array[Byte]],
    refProvider: () ⇒ ValueRef,
    onMRChange: Bytes ⇒ Unit
  ): Try[DatasetStorage] = {

    // todo create direct and faster codec for Long
    implicit val long2bytesCodec: Codec[Task, Array[Byte], ValueRef] = Codec.pure(
      ByteBuffer.wrap(_).getLong(),
      ByteBuffer.allocate(java.lang.Long.BYTES).putLong(_).array()
    )
    import Codec.identityCodec

    RocksDbStore(s"${datasetId}_blob")
      .map(rockDb ⇒ {
        new DatasetStorage(
          MerkleBTree(s"${datasetId}_tree", cryptoHasher),
          KVStore.transform(rockDb),
          MerkleRootCalculator(cryptoHasher),
          refProvider,
          onMRChange
        )
      })

  }

}
