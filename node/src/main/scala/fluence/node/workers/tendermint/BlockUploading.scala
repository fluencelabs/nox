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

import cats.data.EitherT
import cats.effect.concurrent.MVar
import cats.effect.{ConcurrentEffect, ContextShift, Resource, Timer}
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Applicative, Monad}
import com.softwaremill.sttp.SttpBackend
import fluence.effects.ipfs.IpfsClient
import fluence.effects.receipt.storage.KVReceiptStorage
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockHistory, Receipt}
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.{Backoff, EffectError}
import fluence.node.MakeResource
import fluence.node.config.storage.RemoteStorageConfig
import fluence.node.workers.control.{ControlRpc, ControlRpcError}
import fluence.node.workers.{Worker, WorkerServices}
import io.circe.Json

import scala.language.higherKinds

/**
 * Implements continuous uploading process of Tendermint's blocks
 *
 * @param history Description of how to store blocks
 */
class BlockUploading[F[_]: ConcurrentEffect: Timer: ContextShift](history: BlockHistory[F], rootPath: Path)
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
    if (!BlockUploading.Enabled) { // TODO: remove that once BlockUploading is enabled
      return Resource.pure(())
    }

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

    val storedReceipts = storage.retrieve()
    // last block, will evaluate only after all storedReceipts were processed
    val lastBlock = storedReceipts.last.flatMap {
      case Some((height, _)) => fs2.Stream.eval(loadLastBlock(height + 1)).unNone
      case None => fs2.Stream.empty
    }
    val lastBlockReceipt = lastBlock.through(parse).through(upload)

    val blocks = fs2.Stream.eval(loadFirstBlock()) ++ rpc.subscribeNewBlock[F]
    val parsed = blocks.through(parse)
    val receipts = storedReceipts ++ lastBlockReceipt ++ parsed.through(upload)

    MakeResource.concurrentStream(
      receipts,
      name = "BlockUploadingStream"
    )
  }

  private def parseBlock(appId: Long): fs2.Pipe[F, Json, Block] = {
    _.map(Block(_)).map {
      case Left(e) =>
        Applicative[F].pure(logger.error(s"BlockUploading: app $appId failed to parse Tendermint block: $e"))
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
      lastManifestReceipt.take.flatMap { lastReceipt =>
        val uploadF: EitherT[F, EffectError, Receipt] = for {
          vmHash <- control.getVmHash
          receipt <- history.upload(block, vmHash, lastReceipt)
          _ <- receiptStorage.put(block.header.height, receipt).leftMap(identity[EffectError])
        } yield receipt

        // TODO: add health check on this: if error keeps happening, miner should be alerted
        // Retry uploading until forever
        backoff
          .retry(
            uploadF,
            (e: EffectError) =>
              Applicative[F].pure(
                logger.error(s"BlockUploading: app $appId error uploading block ${block.header.height}: $e")
            )
          )
          .map(
            receipt => {
              logger.info(s"BlockUploading: app $appId block ${block.header.height} uploaded")
              receipt
            }
          )
          .flatTap(receipt => lastManifestReceipt.put(Some(receipt)))
      }
    }
  }

  private def loadFirstBlock(): F[Json] = ???
  private def loadLastBlock(lastSavedReceiptHeight: Long): F[Option[Json]] = ???
}

object BlockUploading {

  private val Enabled = false

  def make[F[_]: Monad: ConcurrentEffect: Timer: ContextShift](
    remoteStorageConfig: RemoteStorageConfig,
    rootPath: Path
  )(
    implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]],
    backoff: Backoff[EffectError] = Backoff.default
  ): BlockUploading[F] = {
    // TODO: should I handle remoteStorageConfig.enabled = false?
    val ipfs = new IpfsClient[F](remoteStorageConfig.ipfs.address)
    val history = new BlockHistory[F](ipfs)
    new BlockUploading[F](history, rootPath)
  }
}
