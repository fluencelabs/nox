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

package fluence.bp.uploading

import cats.Applicative
import cats.effect.Resource
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockManifest, Receipt}
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.statemachine.api.command.ReceiptBus

import scala.language.higherKinds

/**
 * Block uploading that does nothing – used to disable block uploading
 */
class DisabledBlockUploading[F[_]: Applicative] extends BlockUploading[F] {

  /**
   * Subscribe on new blocks from tendermint and upload them one by one to the decentralized storage
   * For each block:
   *   1. retrieve vmHash from state machine
   *   2. Send block manifest receipt to state machine
   *
   */
  override def start(
    appId: Long,
    receiptStorage: ReceiptStorage[F],
    subscribeNewBlock: Long => fs2.Stream[F, Block],
    receiptBus: ReceiptBus[F],
    onUploaded: (BlockManifest, Receipt) => F[Unit]
  )(implicit log: Log[F], backoff: Backoff[EffectError]): Resource[F, Unit] = Resource.pure(())
}
