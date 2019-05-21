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

import cats.Monad
import cats.effect.concurrent.MVar
import cats.effect.{Concurrent, ConcurrentEffect, Resource}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.{BlockHistory, Receipt}
import fluence.node.workers.Worker
import scodec.bits.ByteVector

import scala.language.higherKinds

case class BlockUploading[F[_]: ConcurrentEffect](history: BlockHistory[F]) extends slogging.LazyLogging {

  /**
   * Subscribe on new blocks from tendermint and upload them one by one to the decentralized storage
   * For each block: 1. retrieve vmHash from state machine; 2. Send block manifest receipt to state machine
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
                Monad[F].pure(logger.error(s"BlockUploading: failed to parse Tendermint block: ${e.getMessage}"))

              case Right(block) =>
                for {
                  lastReceipt <- lastManifestReceipt.read
                  vmHash <- services.control.getVmHash
                  receipt <- history.upload(block, vmHash, lastReceipt)
                  _ <- services.control.sendBlockReceipt(receipt)
                  _ <- lastManifestReceipt.put(Some(receipt))
                } yield ()
            }
          }.compile.drain
        )
      )(_.cancel)
    } yield ()
  }
}
