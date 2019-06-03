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

import cats.Monad
import cats.data.EitherT
import cats.effect.concurrent.MVar
import cats.effect._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
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
    val upload = uploadBlock(appId, lastManifestReceipt, control, storage)
    def sendReceipt(receipt: Receipt, rType: ReceiptType.Value) = backoff.retry(
      control.sendBlockReceipt(receipt, rType),
      (e: ControlRpcError) => Log[F].error(s"error sending receipt: $e")
    )

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
    val subscriptionBlocks = rpc.subscribeNewBlock[F].through(parse)

    val storedTypedReceipts =
      storedReceipts.dropLast.map(_._2 -> ReceiptType.Stored) ++
        storedReceipts.takeRight(1).map(_._2 -> ReceiptType.LastStored)

    val newReceipts = (lastOrFirstBlock ++ subscriptionBlocks).through(upload).map(_ -> ReceiptType.New)

    val receipts = (storedTypedReceipts ++ newReceipts).evalMap(sendReceipt _ tupled)

    MakeResource.concurrentStream(receipts, name = "BlockUploadingStream")
  }

  private def parseBlock(appId: Long): fs2.Pipe[F, Json, Block] = {
    _.map(Block(_)).map {
      case Left(e) =>
        Log[F].error(s"BlockUploading: app $appId failed to parse Tendermint block: $e")
        None

      case Right(b) => Some(b)
    }.unNone
  }

  private def uploadBlock(
    appId: Long,
    lastManifestReceipt: MVar[F, Option[Receipt]],
    control: ControlRpc[F],
    receiptStorage: KVReceiptStorage[F],
  )(implicit backoff: Backoff[EffectError]): fs2.Pipe[F, Block, Receipt] = {
    _.evalMap { block =>
      def logError[E <: EffectError](e: E) =
        Log[F].error(s"BlockUploading: app $appId error uploading block ${block.header.height}: $e", e)

      val getVmHash =
        backoff.retry(control.getVmHash, logError) <* Log[F].info(s"got vmhash ${block.header.height}")

      (lastManifestReceipt.take, getVmHash).tupled.flatMap {
        case (lastReceipt, vmHash) =>
          val uploadF: EitherT[F, EffectError, Receipt] = for {
            receipt <- history.upload(block, vmHash, lastReceipt)
            _ <- receiptStorage.put(block.header.height, receipt).leftMap(identity[EffectError])
            _ = println(s"saved receipt ${block.header.height}")
          } yield receipt

          // TODO: add health check on this: if error keeps happening, miner should be alerted
          // Retry uploading until forever
          backoff
            .retry(uploadF, logError)
            .flatTap(_ => Log[F].info(s"BlockUploading: app $appId block ${block.header.height} uploaded"))
            .flatTap(receipt => lastManifestReceipt.put(Some(receipt)))
      }
    }
  }

  private def loadFirstBlock(rpc: TendermintRpc[F])(implicit backoff: Backoff[EffectError]): F[Block] = {
    backoff
      .retry(rpc.block(1), (e: RpcError) => Log[F].error(s"BlockUploading load first block: $e"))
  }

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
