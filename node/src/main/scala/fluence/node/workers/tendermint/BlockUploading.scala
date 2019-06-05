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

import cats.data.{Chain, EitherT}
import cats.effect._
import cats.effect.concurrent.MVar
import cats.syntax.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Applicative, Monad}
import com.softwaremill.sttp.SttpBackend
import fluence.effects.ipfs.IpfsClient
import fluence.effects.receipt.storage.KVReceiptStorage
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockHistory, Receipt}
import fluence.effects.tendermint.rpc.{RpcError, TendermintRpc}
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Log, LogFactory}
import fluence.node.MakeResource
import fluence.node.config.storage.RemoteStorageConfig
import fluence.node.workers.Worker
import fluence.node.workers.control.{ControlRpc, ControlRpcError}
import fluence.statemachine.control.ReceiptType
import io.circe.Json

import scala.language.{higherKinds, postfixOps}

/**
 * Implements continuous uploading process of Tendermint's blocks
 *
 * @param history Description of how to store blocks
 */
class BlockUploading[F[_]: ConcurrentEffect: Timer: ContextShift: Log](history: BlockHistory[F], rootPath: Path)
    extends slogging.LazyLogging {

  /**
   * Subscribe on new blocks from tendermint and upload them one by one to the decentralized storage
   * For each block:
   *   1. retrieve vmHash from state machine
   *   2. Send block manifest receipt to state machine
   *
   * @param worker Blocks are coming from this worker's Tendermint; receipts are sent to this worker
   */
  def start(worker: Worker[F])(implicit backoff: Backoff[EffectError] = Backoff.default): Resource[F, Unit] = {
    for {
      receiptStorage <- KVReceiptStorage.make[F](worker.appId, rootPath)
      // Storage for a previous manifest
      lastManifestReceipt <- Resource.liftF(MVar.of[F, Option[Receipt]](None))
      blocks <- worker.services.tendermint.subscribeNewBlock[F]
      _ <- pushReceipts(
        worker.appId,
        lastManifestReceipt,
        receiptStorage,
        worker.services.tendermint,
        worker.services.control,
        blocks
      )
    } yield ()
  }

  private def pushReceipts(
    appId: Long,
    lastManifestReceipt: MVar[F, Option[Receipt]],
    storage: KVReceiptStorage[F],
    rpc: TendermintRpc[F],
    control: ControlRpc[F],
    subscriptionBlocksRaw: fs2.Stream[F, Json]
  )(implicit backoff: Backoff[EffectError], F: Applicative[F]) = {
    val parse = parseBlock(appId)

    def upload(empties: Chain[Receipt], block: Block) =
      uploadBlock(block, empties, appId, lastManifestReceipt, control, storage)
    def uploadEmpty(block: Block) = upload(Chain.empty, block)

    def sendReceipt(receipt: Receipt, rType: ReceiptType.Value) = backoff.retry(
      control.sendBlockReceipt(receipt, rType),
      (e: ControlRpcError) => Log[F].error(s"error sending receipt: $e")
    )

    def emptyBlock(b: Block) = b.data.txs.forall(_.isEmpty)

    /*
     * There are 2 problems solved by `lastOrFirstBlock`:
     *
     * 1. If stored receipts are empty, then we're still on a 1st block
     *    In that case, `subscribeNewBlock` might miss 1st block (it's a race), so we're load it via `loadFirstBlock`,
     *    and send its receipt as a first one.
     *
     * 2. Otherwise, we already had processed some blocks
     *    But it is possible that we had failed to upload and/or store last block. In that case,
     *    we need to load it via `loadLastBlock(lastReceipt.height + 1)`, and send its receipt after all stored receipts
     */

    val storedReceipts = storage.retrieve()
    val lastOrFirstBlock = storedReceipts.last.flatMap {
      case None => fs2.Stream.eval(loadFirstBlock(rpc))
      case Some((height, _)) => fs2.Stream.eval(loadLastBlock(height + 1, rpc)).unNone
    }

    val subscriptionBlocks = subscriptionBlocksRaw
      .through(parse)
      // Skip first block due to race condition (see details above)
      .dropWhile(_.header.height < 2)
      .evalTap(b => Log[F].info(s"subscription block: ${b.header.height}"))

    // Group empty blocks with the first non-empty block, and upload empty blocks right away
    val grouped = (lastOrFirstBlock ++ subscriptionBlocks)
      .evalScan[F, Either[Chain[Receipt], (Chain[Receipt], Block)]](Left(Chain.empty)) {
        case (Left(empties), block) if emptyBlock(block) => uploadEmpty(block).map(r => Left(empties :+ r))
        case (Left(empties), block) => F.pure((empties -> block).asRight)
        case (Right(_), block) if emptyBlock(block) => uploadEmpty(block).map(r => Left(Chain(r)))
        case (Right(_), block) => F.pure((Chain.empty -> block).asRight)
      }

    // Receipts from new blocks (as opposed to stored receipts)
    val newReceipts = grouped.flatMap {
      // Emit receipts for the empty blocks
      case Left(empties) => fs2.Stream.emits(empties.toList.takeRight(1))
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

  private def parseBlock(appId: Long): fs2.Pipe[F, Json, Block] = {
    _.map(Block(_) match {
      case Left(e) =>
        // TODO: retry loading block from RPC until it parses
        Log[F].error(s"app $appId failed to parse Tendermint block: $e")
        None

      case Right(b) => Some(b)
    }).unNone
  }

  private def uploadBlock(
    block: Block,
    emptiesReceipts: Chain[Receipt] = Chain.empty,
    appId: Long,
    lastManifestReceipt: MVar[F, Option[Receipt]],
    control: ControlRpc[F],
    receiptStorage: KVReceiptStorage[F]
  )(implicit backoff: Backoff[EffectError]) =
    Log[F].scope("app" -> appId.toString, "block" -> block.header.height.toString, "upload block" -> "") { log =>
      def logError[E <: EffectError](e: E) = log.error("", e)

      for {
        _ <- log.trace(s"started")
        lastReceipt <- lastManifestReceipt.take
        vmHash <- backoff.retry(control.getVmHash, logError)
        receipt <- backoff.retry(history.upload(block, vmHash, lastReceipt, emptiesReceipts.toList), logError)
        _ <- backoff.retry(receiptStorage.put(block.header.height, receipt), logError)
        _ <- lastManifestReceipt.put(Some(receipt))
        _ <- log.trace(s"finished")
      } yield receipt
    }

  private def loadFirstBlock(rpc: TendermintRpc[F])(implicit backoff: Backoff[EffectError], log: Log[F]): F[Block] =
    backoff.retry(
      rpc.block(1),
      (e: RpcError) => log.error(s"load first block: $e")
    )

  private def loadLastBlock(lastSavedReceiptHeight: Long, rpc: TendermintRpc[F]): F[Option[Block]] =
    // TODO: retry on all errors except 'this block doesn't exist'
    rpc.block(lastSavedReceiptHeight).toOption.value
}

object BlockUploading {

  def make[F[_]: Monad: ConcurrentEffect: Timer: ContextShift: Clock](
    remoteStorageConfig: RemoteStorageConfig,
    rootPath: Path
  )(
    implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]],
    backoff: Backoff[EffectError] = Backoff.default
  ): F[BlockUploading[F]] = {
    // TODO: should I handle remoteStorageConfig.enabled = false?
    val ipfs = new IpfsClient[F](remoteStorageConfig.ipfs.address)
    val history = new BlockHistory[F](ipfs)
    // TODO: Move log creation to MasterNodeApp
    LogFactory.forPrintln[F]().init("BlockUploading", "", Log.Debug).map { implicit log =>
      new BlockUploading[F](history, rootPath)
    }
  }
}
