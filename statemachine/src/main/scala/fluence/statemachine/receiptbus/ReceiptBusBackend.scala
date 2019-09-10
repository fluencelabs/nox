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

package fluence.statemachine.receiptbus

import fluence.statemachine.api.command.ReceiptBus
import cats.instances.long._
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.effect.Concurrent
import fluence.log.Log
import fluence.statemachine.api.data.BlockReceipt
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * [[ReceiptBus]] provides full duplex communication with the Node. [[ReceiptBusBackend]] is the State machine's side
 * of the bus: get [[BlockReceipt]], push VM hash.
 *
 */
trait ReceiptBusBackend[F[_]] {
  def isEnabled: Boolean
  private[statemachine] def getReceipt(height: Long)(implicit log: Log[F]): F[BlockReceipt]
  private[statemachine] def enqueueVmHash(height: Long, hash: ByteVector)(implicit log: Log[F]): F[Unit]
}

object ReceiptBusBackend {

  /**
   * Builds default implementation for [[ReceiptBusBackend]]: see [[QueuesReceiptBusBackend]].
   *
   * The Bus holds some state to pass data from front-end to back-end, thus it's a single object implementing both interfaces.
   *
   * @return
   */
  def apply[F[_]: Concurrent](isEnabled: Boolean): F[ReceiptBus[F] with ReceiptBusBackend[F]] =
    for {
      // getVmHash may be retried by node, so using LastCachingQueue
      hashQueue <- LastCachingQueue[F, VmHash, Long]
      // Using simple queue instead of LastCachingQueue because currently there are no retries on receipts
      receiptQueue <- fs2.concurrent.Queue.unbounded[F, BlockReceipt]
    } yield new QueuesReceiptBusBackend[F](isEnabled, receiptQueue, hashQueue)
}
