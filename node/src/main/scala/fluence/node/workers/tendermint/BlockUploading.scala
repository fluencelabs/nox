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
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Monad, Traverse}
import com.softwaremill.sttp.SttpBackend
import fluence.effects.ipfs.IpfsClient
import fluence.effects.receipt.storage.KVReceiptStorage
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockHistory, Receipt}
import fluence.effects.tendermint.rpc.{RpcError, TendermintRpc}
import fluence.effects.{Backoff, EffectError}
import fluence.log.{Context, Log, PrintlnLog}
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
      _ <- pushReceipts(
        worker.appId,
        lastManifestReceipt,
        receiptStorage,
        worker.services.tendermint,
        worker.services.control,
      )
    } yield ()
  }

  private def pushReceipts(
    appId: Long,
    lastManifestReceipt: MVar[F, Option[Receipt]],
    storage: KVReceiptStorage[F],
    rpc: TendermintRpc[F],
    control: ControlRpc[F],
  )(implicit backoff: Backoff[EffectError]) = {
    // pipes
    val parse = parseBlock(appId)
    def upload = uploadBlocks(appId, lastManifestReceipt, control, storage)
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

    val subscriptionBlocks = rpc
      .subscribeNewBlock[F]
      .through(parse)
      // Skip first block due to race condition (see details above)
      .dropWhile(_.header.height < 2)

    // Group sequence of empty blocks with the first non-empty block (empty := 0 txs)
    val grouped = (lastOrFirstBlock ++ subscriptionBlocks)
      .scan[Either[Chain[Block], (Chain[Block], Block)]](Left(Chain.empty)) {
        case (Left(empties), block) if emptyBlock(block) => Left(empties :+ block)
        case (Left(empties), block) /* if !emptyBlock(block) */ => Right(empties -> block)
        case (Right(_), block) if emptyBlock(block) => Left(Chain(block))
        case (Right(_), block) /* if !emptyBlock(block) */ => Right(Chain.empty -> block)
      }
      .map(_.toOption)
      .unNone

    // Receipts from new blocks (as opposed to stored receipts)
    val newReceipts = grouped.through(upload).map(_ -> ReceiptType.New)

    // Receipts from storage; last one will treated differently (see AbciService for details)
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
        Log[F].error(s"app $appId failed to parse Tendermint block: $e")
        None

      case Right(b) => Some(b)
    }).unNone
  }

  private def uploadBlocks(
    appId: Long,
    lastManifestReceipt: MVar[F, Option[Receipt]],
    control: ControlRpc[F],
    receiptStorage: KVReceiptStorage[F]
  )(implicit backoff: Backoff[EffectError]): fs2.Pipe[F, (Chain[Block], Block), Receipt] = {
    def upload(block: Block, emptiesReceipts: Chain[Receipt] = Chain.empty) = {
      def logError[E <: EffectError](e: E) =
        Log[F].error(s"app $appId upload block ${block.header.height}: $e", e)

      val getVmHash =
        backoff.retry(control.getVmHash, logError) <* Log[F].info(s"got vmhash ${block.header.height}")

      for {
        lastReceipt <- lastManifestReceipt.take
        vmHash <- getVmHash
        receipt <- backoff.retry(history.upload(block, vmHash, lastReceipt, emptiesReceipts.toList), logError)
        _ = println(s"saved receipt ${block.header.height}")
        _ <- backoff.retry(receiptStorage.put(block.header.height, receipt), logError)
        _ <- Log[F].info(s"app $appId block ${block.header.height} uploaded")
        _ <- lastManifestReceipt.put(Some(receipt))
      } yield receipt
    }

    _.evalMap {
      case (empties, block) =>
        Traverse[Chain].traverse(empties)(upload(_)) >>= (upload(block, _))
    }
  }

  private def loadFirstBlock(rpc: TendermintRpc[F])(implicit backoff: Backoff[EffectError]): F[Block] =
    backoff.retry(
      rpc.block(1),
      (e: RpcError) => Log[F].error(s"load first block: $e")
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
  ): BlockUploading[F] = {
    // TODO: should I handle remoteStorageConfig.enabled = false?
    val ipfs = new IpfsClient[F](remoteStorageConfig.ipfs.address)
    val history = new BlockHistory[F](ipfs)
    // TODO: Move log creation to MasterNodeApp
    implicit val ctx = Context.init("BlockUploading", "", Log.Debug)
    implicit val log = new PrintlnLog(ctx)
    new BlockUploading[F](history, rootPath)
  }
}
