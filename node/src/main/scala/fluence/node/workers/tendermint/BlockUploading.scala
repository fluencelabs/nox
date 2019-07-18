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

import cats.data.{Chain, EitherT}
import cats.effect._
import cats.effect.concurrent.MVar
import cats.instances.list._
import cats.syntax.applicative._
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Applicative, Traverse}
import com.softwaremill.sttp.SttpBackend
import fluence.effects.ipfs.IpfsUploader
import fluence.effects.receipt.storage.ReceiptStorage
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockHistory, Receipt}
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.node.MakeResource
import fluence.node.workers.Worker
import fluence.node.workers.control.{ControlRpc, ControlRpcError}
import fluence.statemachine.control.{ReceiptType, VmHash}
import scodec.bits.ByteVector

import scala.language.{higherKinds, postfixOps}

private[tendermint] case class BlockUpload(block: Block,
                                           vmHash: ByteVector,
                                           emptyReceipts: Option[Chain[Receipt]] = None)

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
    def upload(b: BlockUpload) = uploadBlock(b, appId, lastManifestReceipt, storage)

    def sendReceipt(receipt: Receipt, rType: ReceiptType.Value)(implicit log: Log[F]) = backoff.retry(
      control.sendBlockReceipt(receipt, rType),
      (e: ControlRpcError) => log.error(s"error sending receipt: $e")
    )

    def emptyBlock(b: BlockUpload) = b.block.data.txs.forall(_.isEmpty)

    // TODO: what if we have lost all data in receipt storage? Node will need to sync it from the decentralized storage

    // TODO: storedReceipts is calculated 3 times. How to memoize that?
    val storedReceipts = storage.retrieve()

    val lastKnownHeight = storedReceipts.last.map(_.map(_._1).getOrElse(0L))

    // Subscribe on blocks, starting with given last known height
    val blocks = lastKnownHeight >>= rpc.subscribeNewBlock

    // Retrieve vm hash for every block
    val blocksWithVmHash = blocks.evalMap(
      block =>
        backoff
          .retry(control.getVmHash(block.header.height),
                 (e: ControlRpcError) => log.error(s"error retrieving vmHash on height ${block.header.height}: $e"))
          .map(BlockUpload(block, _))
    )

    // Group empty blocks with the first non-empty block; upload empty blocks right away
    val grouped = blocksWithVmHash
      .evalTap(b => log.info(s"processing block ${b.block.header.height}"))
      .evalScan[F, Either[Chain[Receipt], BlockUpload]](Left(Chain.empty)) {
        case (Left(empties), block) if emptyBlock(block) => upload(block).map(r => Left(empties :+ r))
        case (Left(empties), block)                      => F.pure(block.copy(emptyReceipts = Some(empties)).asRight)
        case (Right(_), block) if emptyBlock(block)      => upload(block).map(r => Left(Chain(r)))
        case (Right(_), block)                           => F.pure(block.asRight)
      }

    // TODO: send these receipts to Kademlia
    // Receipts from the new blocks (as opposed to stored receipts)
    val newReceipts = grouped.flatMap {
      // Emit receipts for the empty blocks
      case Left(empties) => fs2.Stream.emits(empties.toList.takeRight(1))
      case Right(block)  => fs2.Stream.eval(upload(block))
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
    block: BlockUpload,
    appId: Long,
    lastManifestReceipt: MVar[F, Option[Receipt]],
    receiptStorage: ReceiptStorage[F]
  )(implicit backoff: Backoff[EffectError], log: Log[F]): F[Receipt] =
    log.scope("block" -> block.block.header.height.toString, "upload block" -> "") { log =>
      def logError[E <: EffectError](e: E) = log.error("", e)

      def upload(b: BlockUpload, r: Option[Receipt]) =
        backoff.retry(history.upload(b.block, b.vmHash, r, b.emptyReceipts.map(_.toList).getOrElse(Nil))(log), logError)

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
