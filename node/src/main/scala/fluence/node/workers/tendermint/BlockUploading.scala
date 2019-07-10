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

import java.nio.ByteBuffer
import java.nio.file.Path

import cats.Applicative
import cats.data.{Chain, EitherT}
import cats.effect._
import cats.effect.concurrent.MVar
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import com.softwaremill.sttp.SttpBackend
import fluence.effects.ipfs.{IpfsClient, IpfsUploader}
import fluence.effects.receipt.storage.{KVReceiptStorage, ReceiptStorage}
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockHistory, Receipt}
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.node.MakeResource
import fluence.node.config.storage.RemoteStorageConfig
import fluence.node.workers.Worker
import fluence.node.workers.control.{ControlRpc, ControlRpcError}
import fluence.statemachine.control.ReceiptType

import scala.language.{higherKinds, postfixOps}

/**
 * Implements continuous uploading process of Tendermint's blocks
 *
 * @param history Description of how to store blocks
 */
class BlockUploading[F[_]: ConcurrentEffect: Timer: ContextShift](
  history: BlockHistory[F],
  receiptStorage: (Long) => Resource[F, ReceiptStorage[F]]
) {

  /**
   * Subscribe on new blocks from tendermint and upload them one by one to the decentralized storage
   * For each block:
   *   1. retrieve vmHash from state machine
   *   2. Send block manifest receipt to state machine
   *
   * @param worker Blocks are coming from this worker's Tendermint; receipts are sent to this worker
   */
  def start(
    worker: Worker[F]
  )(implicit log: Log[F], backoff: Backoff[EffectError] = Backoff.default): Resource[F, Unit] = {
    for {
      receiptStorage <- receiptStorage(worker.appId)
      // Storage for a previous manifest
      lastManifestReceipt <- Resource.liftF(MVar.of[F, Option[Receipt]](None))
      _ <- pushReceipts(
        worker.appId,
        lastManifestReceipt,
        receiptStorage,
        worker.services.tendermint,
        worker.services.control
      )
    } yield ()
  }

  private def pushReceipts(
    appId: Long,
    lastManifestReceipt: MVar[F, Option[Receipt]],
    storage: ReceiptStorage[F],
    rpc: TendermintRpc[F],
    control: ControlRpc[F]
  )(implicit backoff: Backoff[EffectError], F: Applicative[F], log: Log[F]) = {
    def upload(empties: Chain[Receipt], block: Block) =
      uploadBlock(block, empties, appId, lastManifestReceipt, control, storage)
    def uploadEmpty(block: Block) = upload(Chain.empty, block)

    def sendReceipt(receipt: Receipt, rType: ReceiptType.Value)(implicit log: Log[F]) = backoff.retry(
      control.sendBlockReceipt(receipt, rType),
      (e: ControlRpcError) => log.error(s"error sending receipt: $e")
    )

    def emptyBlock(b: Block) = b.data.txs.forall(_.isEmpty)

    // TODO: storedReceipts got calculated 4 times. How to memoize that?
    val storedReceipts =
      fs2.Stream.eval(log.info("will start loading stored receipts")) >>
        storage
          .retrieve()
          .evalTap(t => log.info(s"stored receipt ${t._1}"))

    val lastKnownHeight = storedReceipts.last.map(_.map(_._1).getOrElse(0L))

    // Skip first block due to race condition (see details above)
    val subscriptionBlocks = lastKnownHeight >>= rpc.subscribeNewBlock

    // Group empty blocks with the first non-empty block; upload empty blocks right away
    // TODO: zip each block with an according vmHash, so the connection between them is more visible and straightforward
    val grouped = subscriptionBlocks
      .evalTap(b => log.info(s"processing block ${b.header.height}"))
      .evalScan[F, Either[Chain[Receipt], (Chain[Receipt], Block)]](Left(Chain.empty)) {
        case (Left(empties), block) if emptyBlock(block) => uploadEmpty(block).map(r => Left(empties :+ r))
        case (Left(empties), block)                      => F.pure((empties -> block).asRight)
        case (Right(_), block) if emptyBlock(block)      => uploadEmpty(block).map(r => Left(Chain(r)))
        case (Right(_), block)                           => F.pure((Chain.empty -> block).asRight)
      }

    // Receipts from the new blocks (as opposed to stored receipts)
    val newReceipts = grouped.flatMap {
      // Emit receipts for the empty blocks
      case Left(empties)           => fs2.Stream.emits(empties.toList.takeRight(1))
      case Right((empties, block)) => fs2.Stream.eval(upload(empties, block))
    }.map(_ -> ReceiptType.New)

    // Receipts from storage; last one will be treated differently, see AbciService for details
    val storedTypedReceipts =
      storedReceipts.dropLast.map(_._2 -> ReceiptType.Stored) ++
        storedReceipts.takeRight(1).map(_._2 -> ReceiptType.LastStored)

    // Send receipts to the state machine (worker)
    val receipts = (storedTypedReceipts ++ newReceipts).evalMap(sendReceipt _ tupled)

    MakeResource.concurrentStream(receipts, name = "BlockUploadingStream")
  }

  private def uploadBlock(
    block: Block,
    emptiesReceipts: Chain[Receipt] = Chain.empty,
    appId: Long,
    lastManifestReceipt: MVar[F, Option[Receipt]],
    // TODO: pass here vmHash instead of ControlRpc - arguments should all be data (except for mvar and storage)
    // TODO: zip each block with an according vmHash, so the connection between them is more visible and straightforward
    control: ControlRpc[F],
    receiptStorage: ReceiptStorage[F]
  )(implicit backoff: Backoff[EffectError], log: Log[F]) =
    log.scope("block" -> block.header.height.toString, "upload block" -> "") { log =>
      def logError[E <: EffectError](e: E) = log.error("", e)

      for {
        _ <- log.debug(s"started")
        lastReceipt <- lastManifestReceipt.take
        vmHash <- backoff.retry(control.getVmHash, logError)
        receipt <- backoff.retry(history.upload(block, vmHash, lastReceipt, emptiesReceipts.toList)(log), logError)
        _ <- backoff.retry(receiptStorage.put(block.header.height, receipt), logError)
        _ <- lastManifestReceipt.put(Some(receipt))
        _ <- log.debug(s"finished")
      } yield receipt
    }
}

object BlockUploading {

  def make[F[_]: Log: ConcurrentEffect: Timer: ContextShift: Clock](
    ipfs: IpfsUploader[F],
    receiptStorage: (Long) => Resource[F, ReceiptStorage[F]]
  )(
    implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]],
    backoff: Backoff[EffectError] = Backoff.default
  ): BlockUploading[F] = {
    // TODO: should I handle remoteStorageConfig.enabled = false?
    val history = new BlockHistory[F](ipfs)
    new BlockUploading[F](history, receiptStorage)
  }
}
