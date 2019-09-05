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

package fluence.node.workers.tendermint.block

import cats.data.Chain
import cats.effect._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.applicative._
import fluence.effects.ipfs.IpfsUploader
import fluence.effects.sttp.SttpStreamEffect
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockHistory, Receipt}
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.node.workers.{Worker, WorkerServices}
import scodec.bits.ByteVector

import scala.language.{higherKinds, postfixOps}

private[tendermint] case class BlockUpload(
  block: Block,
  vmHash: ByteVector,
  emptyReceipts: Option[Chain[Receipt]] = None
)

trait BlockUploading[F[_]] {

  /**
   * Subscribe on new blocks from tendermint and upload them one by one to the decentralized storage
   * For each block:
   *   1. retrieve vmHash from state machine
   *   2. Send block manifest receipt to state machine
   *
   */
  // TODO: separate block uploading into replay and usual block processing parts, so replay could be handled without a need
  //  for RPC and WRPC. Should be possible to handle block replay, wait until Tendermint started RPC, and then
  //  connect to Websocket and create blockstore after everything is initialized
  def start(
    appId: Long,
    services: WorkerServices[F]
  )(implicit log: Log[F], backoff: Backoff[EffectError]): Resource[F, Unit]
}

object BlockUploading {

  def apply[F[_]: ConcurrentEffect: Timer: ContextShift: SttpStreamEffect](
    enabled: Boolean,
    ipfs: => IpfsUploader[F]
  )(
    implicit
    backoff: Backoff[EffectError] = Backoff.default,
    log: Log[F]
  ): Resource[F, BlockUploading[F]] =
    if (enabled) {
      val history = new BlockHistory[F](ipfs)
      Log[F].scope("block-uploading") { implicit log: Log[F] =>
        (new BlockUploadingImpl[F](history): BlockUploading[F]).pure[Resource[F, ?]]
      }
    } else {
      Log
        .resource[F]
        .info("Block uploading disabled")
        .as(
          new DisabledBlockUploading[F]
        )
    }
}
