/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.effects.receipt.storage

import java.nio.file.Path

import cats.Monad
import cats.data.EitherT
import cats.effect.{ContextShift, LiftIO, Resource, Sync}
import fluence.effects.kvstore.KVStore
import fluence.effects.tendermint.block.history.Receipt
import fluence.kad.protocol.Key
import fluence.log.Log

import scala.language.higherKinds

/**
 * Algebra for storage of block receipts.
 */
trait ReceiptStorage[F[_]] {
  val appId: Long

  /**
   * Stores receipt for the specified app at a given height.
   *
   * @param height block height for the receipt
   * @param receipt block receipt
   */
  def put(height: Long, receipt: Receipt)(implicit log: Log[F]): EitherT[F, ReceiptStorageError, Unit]

  /**
   * Gets a receipt for specified app and height
   *
   * @param height block height
   * @return receipt if exists
   */
  def get(height: Long)(implicit log: Log[F]): EitherT[F, ReceiptStorageError, Option[Receipt]]

  /**
   * Retrieves a chain of receipts, starting at block height `from`, until `to`.
   *
   * @param from height of the first receipt in the resulting chain
   * @param to height of the last receipt in the resulting chain
   * @return chain of receipts
   */
  def retrieve(
    from: Option[Long] = None,
    to: Option[Long] = None
  )(implicit log: Log[F]): fs2.Stream[F, (Long, Receipt)]
}

object ReceiptStorage {

  /**
   * Creates instance of ReceiptStorage backed by local file system and RocksDB
   */
  def local[F[_]: Sync: LiftIO: ContextShift: Log](appId: Long, rootPath: Path): Resource[F, ReceiptStorage[F]] =
    KVReceiptStorage.make(appId, rootPath)

  /**
   * Creates instance of ReceiptStorage backed by Kademlia DHT
   */
  def dht[F[_]: Monad](appId: Long, kv: KVStore[F, Key, Receipt]): Resource[F, DhtReceiptStorage[F]] =
    Resource.pure(new DhtReceiptStorage[F](appId, kv))
}
