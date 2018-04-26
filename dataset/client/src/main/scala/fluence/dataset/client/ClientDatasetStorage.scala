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

import cats.effect.IO
import cats.syntax.profunctor._
import fluence.btree.client.MerkleBTreeClient.ClientState
import fluence.btree.client.{MerkleBTreeClient, MerkleBTreeClientApi}
import fluence.btree.core.Hash
import fluence.crypto.Crypto
import fluence.crypto.signature.Signer
import fluence.dataset.protocol.DatasetStorageRpc
import monix.eval.Task
import monix.execution.atomic.Atomic
import monix.reactive.Observable

import Ordered._
import scala.language.higherKinds

//todo datasetId - kademlia key
/**
 * Dataset storage that allows save and retrieve some value by key from remote dataset.
 *
 * @param datasetId Dataset ID
 * @param datasetStartVer Dataset version at the moment of creating this storage
 * @param bTreeIndex An interface to dataset index.
 * @param storageRpc Remotely-accessible interface to all dataset storage operation.
 * @param valueCrypt Encrypting/decrypting provider for ''Value''
 * @param hasher     Hash provider
 *
 * @tparam K The type of keys
 * @tparam V The type of stored values
 */
// todo unit test
class ClientDatasetStorage[K, V](
  datasetId: Array[Byte],
  datasetStartVer: Long,
  bTreeIndex: MerkleBTreeClientApi[Task, K],
  storageRpc: DatasetStorageRpc[Task, Observable],
  keyCrypt: Crypto.Cipher[K],
  valueCrypt: Crypto.Cipher[V],
  hasher: Crypto.Hasher[Array[Byte], Hash]
)(implicit ord: Ordering[K])
    extends ClientDatasetStorageApi[Task, Observable, K, V] with slogging.LazyLogging {

  private val datasetVer = IO.pure(Atomic(datasetStartVer))

  override def get(key: K): Task[Option[V]] =
    for {
      getCallbacks ← bTreeIndex.initGet(key)
      version ← Task.fromIO(datasetVer).map(_.get)
      serverResponse ← storageRpc.get(datasetId, version, getCallbacks).doOnFinish { _ ⇒
        getCallbacks.recoverState()
      }

      resp ← decryptOption(serverResponse)
    } yield resp

  override def range(from: K, to: K): Observable[(K, V)] =
    for {
      _ ← validateRange(from, to)
      rangeCallbacks ← Observable.fromTask(bTreeIndex.initRange(from))
      version ← Observable.fromIO(datasetVer).map(_.get)
      pair ← storageRpc
        .range(datasetId, version, rangeCallbacks)
        .doAfterTerminateTask { _ ⇒
          rangeCallbacks.recoverState()
        }
        // decrypt key
        .mapTask {
          case (encKey, encValue) ⇒
            keyCrypt.inverse
              .runF[Task](encKey)
              .map(plainKey ⇒ plainKey → encValue)
        }
        // check key upper bound
        .takeWhile { case (key, _) ⇒ key <= to }
        // decrypt value
        .mapTask {
          case (plainKey, encValue) ⇒
            valueCrypt.inverse
              .runF[Task](encValue)
              .map(plainValue ⇒ plainKey → plainValue)
        }

    } yield {
      logger.trace(s"Client receive $pair for range query from=$from, to=$to ")
      pair
    }

  override def put(key: K, value: V): Task[Option[V]] =
    for {
      encValue ← valueCrypt.direct.runF[Task](value)
      encValueHash ← hasher.runF[Task](encValue)
      version ← Task.fromIO(datasetVer).map(_.get)
      putCallbacks ← bTreeIndex.initPut(key, encValueHash, version)
      serverResponse ← storageRpc
        .put(datasetId, version, putCallbacks, encValue)
        .doOnFinish {
          // in error case we should return old value of clientState back
          case Some(e) ⇒ putCallbacks.recoverState()
          case _ ⇒ Task.unit
        }

      _ ← Task.fromIO(datasetVer).map(_.increment())
      resp ← decryptOption(serverResponse)
    } yield resp

  override def remove(key: K): Task[Option[V]] =
    for {
      version ← Task.fromIO(datasetVer).map(_.get)
      removeCmd ← bTreeIndex.initRemove(key, version)
      serverResponse ← storageRpc.remove(datasetId, version, removeCmd)
      _ ← Task.fromIO(datasetVer).map(_.increment())
      resp ← decryptOption(serverResponse)
    } yield resp

  private def decryptOption(response: Option[Array[Byte]]): Task[Option[V]] =
    response match {
      case Some(r) ⇒ valueCrypt.inverse.runF[Task](r).map(Option.apply)
      case None ⇒ Task(None)
    }

  private def validateRange(from: K, to: K): Observable[Unit] =
    if (from > to)
      Observable.raiseError(
        new IllegalArgumentException(
          "Reverse order traversal is not yet supported, range start point should be less or equals than end point"
        )
      )
    else Observable(())

}

object ClientDatasetStorage {

  /**
   * Creates Dataset storage that allows save and retrieve some value by key from remote dataset.
   *
   * @param datasetId Dataset ID
   * @param datasetStartVer Dataset version at the moment of creating this storage
   * @param hasher Hash provider
   * @param storageRpc Remotely-accessible interface to all dataset storage operation.
   * @param keyCrypt Encrypting/decrypting provider for ''key''
   * @param valueCrypt Encrypting/decrypting provider for ''Value''
   * @param clientState Initial client state, includes merkle root for dataset. For new dataset should be ''None''
   * @param signer          Algorithm to produce signatures. Used for sealing execState by contract owner
   *
   * @param ord         The ordering to be used to compare keys.
   *
   * @tparam K The type of keys
   * @tparam V The type of stored values
   */
  def apply[K, V](
    datasetId: Array[Byte],
    datasetStartVer: Long,
    hasher: Crypto.Hasher[Array[Byte], Array[Byte]],
    storageRpc: DatasetStorageRpc[Task, Observable],
    keyCrypt: Crypto.Cipher[K],
    valueCrypt: Crypto.Cipher[V],
    clientState: Option[ClientState],
    signer: Signer
  )(implicit ord: Ordering[K]): ClientDatasetStorage[K, V] = {

    val wrappedHasher = hasher.rmap(Hash(_))

    val bTreeIndex = MerkleBTreeClient(clientState, keyCrypt, wrappedHasher, signer)
    new ClientDatasetStorage[K, V](
      datasetId,
      datasetStartVer,
      bTreeIndex,
      storageRpc,
      keyCrypt,
      valueCrypt,
      wrappedHasher
    )
  }

}
