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

package fluence.statemachine.hashesbus

import fluence.statemachine.api.command.HashesBus
import cats.instances.long._
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.flatMap._
import HasOrderedProperty.syntax._
import cats.Monad
import cats.data.EitherT
import cats.effect.Concurrent
import fluence.effects.EffectError
import fluence.log.Log
import fluence.statemachine.api.data.BlockReceipt
import scodec.bits.ByteVector

import scala.language.higherKinds

class HashesBusBackend[F[_]: Monad](
  // Using simple queue instead of LastCachingQueue because currently there are no retries on receipts
  private val receiptQueue: fs2.concurrent.Queue[F, BlockReceipt],
  // getVmHash may be retried by node, so using LastCachingQueue
  private val hashQueue: LastCachingQueue[F, VmHash, Long]
) extends HashesBus[F] {
  private def traceBU(msg: String)(implicit log: Log[F]) = Log[F].trace(Console.YELLOW + "BUD: " + msg + Console.RESET)

  /**
   * Retrieves a single vm hash from queue. Called by node on block manifest uploading.
   * Async-ly blocks until there's a vmHash with specified height
   */
  override def getVmHash(height: Long)(implicit log: Log[F]): EitherT[F, EffectError, ByteVector] =
    EitherT.right(traceBU(s"getVmHash $height") *> hashQueue.dequeue(height)).map(_.hash)

  /**
   * Stores block receipt in memory, async blocks if previous receipt is still there
   * Receipt comes from node through control rpc
   *
   * @param receipt Receipt to store
   */
  override def sendBlockReceipt(receipt: BlockReceipt)(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
    EitherT.right(traceBU(s"enqueueReceipt ${receipt.height}") *> receiptQueue.enqueue1(receipt))

  /**
   * Retrieves block receipt, async-ly blocks until there's a receipt with specified height
   */
  private[statemachine] def getReceipt(height: Long)(implicit log: Log[F]): F[BlockReceipt] =
    traceBU(s"getReceipt $height") *> receiptQueue.dequeueByBoundary(height)

  /**
   * Adds vm hash to queue, so node can retrieve it for block manifest uploading
   */
  private[statemachine] def enqueueVmHash(height: Long, hash: ByteVector)(implicit log: Log[F]): F[Unit] =
    traceBU(s"enqueueVmHash $height") *> hashQueue.enqueue1(VmHash(height, hash))
}

object HashesBusBackend {
  private[statemachine] def apply[F[_]: Concurrent]: F[HashesBusBackend[F]] =
    for {
      // getVmHash may be retried by node, so using LastCachingQueue
      hashQueue <- LastCachingQueue[F, VmHash, Long]
      // Using simple queue instead of LastCachingQueue because currently there are no retries on receipts
      receiptQueue <- fs2.concurrent.Queue.unbounded[F, BlockReceipt]
    } yield new HashesBusBackend[F](receiptQueue, hashQueue)
}
