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
import fluence.effects.tendermint.block.history.Receipt
import fluence.node.workers.Worker

import scala.language.higherKinds

object BlockUploading extends slogging.LazyLogging {

  def start[F[_]: ConcurrentEffect](worker: Worker[F]): Resource[F, Unit] = {
    def upload(lastReceipt: Option[Receipt], block: Block): F[Receipt] = ???

    val services = worker.services

    for {
      lastManifestReceipt <- Resource.liftF(MVar.of[F, Option[Receipt]](None))
      blocks <- services.tendermint.subscribeNewBlock[F]
      _ <- Resource.make(
        Concurrent[F].start(
          blocks.evalMap { blockRaw =>
            Block(blockRaw) match {
              case Left(e) =>
                Monad[F].pure(logger.error(s"DWS failed to parse Tendermint block: ${e.getMessage}"))

              case Right(block) =>
                for {
                  lastReceipt <- lastManifestReceipt.read
                  receipt <- upload(lastReceipt, block)
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
