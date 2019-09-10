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

import cats.Applicative
import cats.data.Chain
import cats.effect._
import cats.effect.concurrent.{Deferred, MVar, Ref}
import cats.syntax.apply._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockHistory, BlockManifest, Receipt}
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.tendermint.rpc.websocket.TendermintWebsocketRpc
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.node.MakeResource
import fluence.node.workers.WorkerServices
import fluence.statemachine.api.command.ReceiptBus
import fluence.statemachine.api.data.BlockReceipt

import scala.language.{higherKinds, postfixOps}

/**
 * Implements continuous uploading process of Tendermint's blocks
 *
 * @param history Description of how to store blocks
 */
class BlockUploadingImpl[F[_]: ConcurrentEffect: Timer: ContextShift](
  history: BlockHistory[F]
) extends BlockUploading[F] {

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
  )(implicit log: Log[F], backoff: Backoff[EffectError] = Backoff.default): Resource[F, Unit] = {
    for {
      // Storage for a previous manifest
      lastManifestReceipt <- Resource.liftF(MVar.of[F, Option[Receipt]](None))
      _ <- pushReceipts(
        appId,
        lastManifestReceipt,
        services.blockManifests.receiptStorage,
        services.tendermintRpc,
        services.tendermintWRpc,
        services.receiptBus,
        services.blockManifests.onUploaded
      )
    } yield ()
  }

  // TODO write docs
  private def pushReceipts(
    appId: Long,
    lastManifestReceipt: MVar[F, Option[Receipt]],
    storage: ReceiptStorage[F],
    rpc: TendermintHttpRpc[F],
    wrpc: TendermintWebsocketRpc[F],
    receiptBus: ReceiptBus[F],
    onManifestUploaded: (BlockManifest, Receipt) ⇒ F[Unit]
  )(implicit backoff: Backoff[EffectError], F: Applicative[F], log: Log[F]): Resource[F, Unit] =
    Resource.liftF((Ref.of[F, Long](0), Deferred[F, Long]).tupled).flatMap {
      case (lastHeightRef, lastHeightDef) ⇒
        def upload(b: BlockUpload) = uploadBlock(b, appId, lastManifestReceipt, storage, onManifestUploaded)
        val sendReceipt = this.sendReceipt(_, receiptBus)

        // TODO: what if we have lost all data in receipt storage? Node will need to sync it from the decentralized storage
        val storedReceipts = getStoredReceipts(storage, lastHeightRef, lastHeightDef)

        // TODO get last known height from the last received receipt
        val lastKnownHeight = fs2.Stream.eval(lastHeightDef.get)

        // Subscribe on blocks, starting with given last known height
        val blocks = lastKnownHeight >>= wrpc.subscribeNewBlock

        // Retrieve vm hash for every block
        val blocksWithVmHash = getBlocksWithVmHashes(blocks, receiptBus)

        // Upload blocks in groups (empty + non-empty)
        val newReceipts = uploadBlocks(blocksWithVmHash, upload)

        // Send receipts to the state machine (worker)
        val receipts = (storedReceipts ++ newReceipts).evalMap(sendReceipt)

        MakeResource.concurrentStream(receipts, name = "BlockUploadingStream")
    }

  private def getStoredReceipts(
    storage: ReceiptStorage[F],
    lastHeightRef: Ref[F, Long],
    lastHeightDef: Deferred[F, Long]
  )(
    implicit log: Log[F]
  ) =
    fs2.Stream.eval(traceBU(s"will start loading stored receipts")) >>
      storage
        .retrieve()
        .evalTap(t => traceBU(s"stored receipt ${t._1}"))
        .evalTap {
          case (h, _) ⇒
            // For each receipt, store its height into a Ref. Note: we assume that receipts come in order
            lastHeightRef.set(h)
        }
        .onFinalize(
          // When stream completed, resolve last height's deferred with the last noticed height
          lastHeightRef.get >>= lastHeightDef.complete
        )
        // Without .scope, onFinalize might be not called in time
        .scope
        .map(_._2)

  private def sendReceipt(receipt: Receipt, receiptBus: ReceiptBus[F])(
    implicit log: Log[F],
    backoff: Backoff[EffectError]
  ) = backoff.retry(
    receiptBus.sendBlockReceipt(BlockReceipt(receipt.height, receipt.jsonBytes())),
    (e: EffectError) => log.error(s"error sending receipt: $e")
  )

  private def getBlocksWithVmHashes(
    blocks: fs2.Stream[F, Block],
    receiptBus: ReceiptBus[F]
  )(implicit backoff: Backoff[EffectError], log: Log[F]) =
    blocks
      .evalTap(b => traceBU(s"got block ${b.header.height}"))
      .evalMap(
        block =>
          backoff
            .retry(
              receiptBus.getVmHash(block.header.height),
              (e: EffectError) => log.error(s"error retrieving vmHash on height ${block.header.height}: $e")
            )
            .map(BlockUpload(block, _))
      )
      .evalTap(b => traceBU(s"got vmHash ${b.block.header.height}"))

  private def uploadBlocks(
    blocks: fs2.Stream[F, BlockUpload],
    upload: BlockUpload => F[Receipt]
  )(implicit log: Log[F], F: Applicative[F]) = {
    def emptyBlock(b: BlockUpload) = b.block.data.txs.forall(_.isEmpty)

    // Group empty blocks with the first non-empty block; upload empty blocks right away and return
    blocks
      .evalTap(b => log.info(s"processing block ${b.block.header.height}"))
      .evalScan[F, Either[Chain[Receipt], BlockUpload]](Left(Chain.empty)) {
        case (Left(empties), block) if emptyBlock(block) => upload(block).map(r => Left(empties :+ r))
        case (Left(empties), block)                      => F.pure(block.copy(emptyReceipts = Some(empties)).asRight)
        case (Right(_), block) if emptyBlock(block)      => upload(block).map(r => Left(Chain(r)))
        case (Right(_), block)                           => F.pure(block.asRight)
      }
      .flatMap {
        // Emit receipts for the empty blocks
        case Left(empties) => fs2.Stream.emits(empties.toList.takeRight(1))
        case Right(block)  => fs2.Stream.eval(upload(block))
      }
  }

  // TODO write docs
  private def uploadBlock(
    block: BlockUpload,
    appId: Long,
    lastManifestReceipt: MVar[F, Option[Receipt]],
    receiptStorage: ReceiptStorage[F],
    onManifestUploaded: (BlockManifest, Receipt) ⇒ F[Unit]
  )(implicit backoff: Backoff[EffectError], log: Log[F]): F[Receipt] =
    log.scope("block" -> block.block.header.height.toString, "upload block" -> "") { implicit log: Log[F] =>
      // Shorthand for logging errors on retry
      def logError[E <: EffectError](e: E) = log.error("", e)

      def upload(b: BlockUpload, r: Option[Receipt]) =
        backoff.retry(
          history.upload(b.block, b.vmHash, r, b.emptyReceipts.map(_.toList).getOrElse(Nil), onManifestUploaded)(log),
          logError
        )

      def storeReceipt(height: Long, receipt: Receipt) = backoff.retry(receiptStorage.put(height, receipt), logError)

      for {
        _ <- log.debug(s"started")
        lastReceipt <- lastManifestReceipt.take
        receipt <- upload(block, lastReceipt)
        _ <- storeReceipt(block.block.header.height, receipt)
        _ <- lastManifestReceipt.put(Some(receipt))
        _ <- log.debug(s"finished")
      } yield receipt
    }

  // Writes a trace log about block uploading
  private def traceBU(msg: String)(implicit log: Log[F]) =
    log.trace(Console.YELLOW + s"BUD: $msg" + Console.RESET)
}
