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

import cats.data.EitherT
import cats.effect.concurrent.MVar
import cats.effect.{Concurrent, ConcurrentEffect, Resource, Timer}
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Applicative, Monad}
import com.softwaremill.sttp.SttpBackend
import fluence.effects.ipfs.IpfsClient
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockHistory, Receipt}
import fluence.effects.{Backoff, EffectError}
import fluence.node.config.storage.RemoteStorageConfig
import fluence.node.workers.Worker

import scala.language.higherKinds

class BlockUploading[F[_]: ConcurrentEffect: Timer](history: BlockHistory[F]) extends slogging.LazyLogging {

  /**
   * Subscribe on new blocks from tendermint and upload them one by one to the decentralized storage
   * For each block:
   *   1. retrieve vmHash from state machine
   *   2. Send block manifest receipt to state machine
   *
   * @param worker Blocks are coming from this worker's Tendermint; receipts are sent to this worker
   */
  def start(worker: Worker[F]): Resource[F, Unit] = {
    val services = worker.services

    for {
      // Storage for a previous manifest
      lastManifestReceipt <- Resource.liftF(MVar.of[F, Option[Receipt]](None))
      blocks <- services.tendermint.subscribeNewBlock[F]
      _ <- Resource.make(
        // Start block processing in a background fiber
        Concurrent[F].start(
          blocks.evalMap { blockRaw =>
            // Parse block from JSON
            Block(blockRaw) match {
              case Left(e) =>
                Applicative[F].pure(logger.error(s"BlockUploading: failed to parse Tendermint block: ${e.getMessage}"))

              case Right(block) =>
                val processF = for {
                  lastReceipt <- EitherT.liftF(lastManifestReceipt.read)
                  vmHash <- services.control.getVmHash
                  receipt <- history.upload(block, vmHash, lastReceipt)
                  _ <- services.control.sendBlockReceipt(receipt)
                  // TODO: How to avoid specifying [F, NoStackTrace, Unit] in liftF?
                  _ <- EitherT.liftF[F, EffectError, Unit](lastManifestReceipt.put(Some(receipt)))
                } yield ()

                // TODO: add health check on this: if error keeps happening, miner should be alerted
                // Retry uploading until forever
                Backoff.default
                  .retry(
                    processF,
                    (e: EffectError) =>
                      Applicative[F].pure(
                        logger.error(s"BlockUploading: Error uploading block ${block.header.height}: ${e.getMessage}")
                    )
                  )
                  .map(
                    _ => logger.info(s"BlockUploading: Block ${block.header.height} uploaded")
                  )
            }
          }.compile.drain
        )
      )(_.cancel)
    } yield ()
  }
}

object BlockUploading {

  def make[F[_]: Monad: ConcurrentEffect: Timer](
    remoteStorageConfig: RemoteStorageConfig
  )(implicit sttpBackend: SttpBackend[EitherT[F, Throwable, ?], fs2.Stream[F, ByteBuffer]]): BlockUploading[F] = {
    // TODO: handle remoteStorageConfig.enabled = false
    val ipfs = new IpfsClient[F](remoteStorageConfig.ipfs.address)
    val history = new BlockHistory[F](ipfs)
    new BlockUploading[F](history)
  }
}
