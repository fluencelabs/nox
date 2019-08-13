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

package fluence.node.workers.tendermint

import cats.Monad
import cats.data.EitherT
import cats.syntax.flatMap._
import fluence.effects.kvstore.{KVReadError, KVStore, KVWriteError, KeyCodecError}
import fluence.effects.receipt.storage.{GetError, PutError, ReceiptStorage, ReceiptStorageError}
import fluence.effects.tendermint.block.history.Receipt
import fluence.kad.protocol.Key
import fluence.log.Log
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * ReceiptStorage for an app, using [[fluence.kad.dht.Dht]] as a storage backend.
 *
 * @param appId Application ID
 * @param dht Dht
 * @tparam F Effect
 */
class DhtReceiptStorage[F[_]: Monad](
  override val appId: Long,
  dht: KVStore[F, Key, Receipt]
) extends ReceiptStorage[F] {

  // Convert appId and height to Kademlia Key
  private def key[E >: KVWriteError with KVReadError](height: Long): EitherT[F, E, Key] =
    Key
      .sha1[F](
        (ByteVector.fromLong(appId) ++ ByteVector.fromLong(height)).toArray
      )
      .leftMap(KeyCodecError)

  /**
   * Stores receipt for the specified app at a given height.
   *
   * @param height  block height for the receipt
   * @param receipt block receipt
   */
  override def put(height: Long, receipt: Receipt)(implicit log: Log[F]): EitherT[F, ReceiptStorageError, Unit] =
    (
      key[KVWriteError](height) >>= (k ⇒ dht.put(k, receipt))
    ).leftMap(PutError(appId, height, _))

  /**
   * Gets a receipt for specified app and height
   *
   * @param height block height
   * @return receipt if exists
   */
  override def get(height: Long)(implicit log: Log[F]): EitherT[F, ReceiptStorageError, Option[Receipt]] =
    (
      key[KVReadError](height) >>= (k ⇒ dht.get(k))
    ).leftMap(GetError(appId, height, _))

  /**
   * Retrieves a chain of receipts, starting at block height `from`, until `to`.
   *
   * @param from height of the first receipt in the resulting chain
   * @param to   height of the last receipt in the resulting chain
   * @return chain of receipts
   */
  override def retrieve(from: Option[Long], to: Option[Long])(implicit log: Log[F]): fs2.Stream[F, (Long, Receipt)] =
    fs2.Stream
      .iterate(from.getOrElse(0l))(_ + 1)
      .filter(h ⇒ to.fold(true)(h <= _))
      .evalMap(
        get(_).value
      )
      .map(_.toOption.flatten)
      .unNoneTerminate
      .map(r ⇒ r.height -> r)
}
