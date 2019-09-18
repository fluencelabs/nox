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

package fluence.bp.embedded

import cats.Monad
import cats.data.EitherT
import cats.effect.Concurrent
import cats.effect.concurrent.Ref
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.bp.api.BlockProducer
import fluence.bp.tx.TxResponse
import fluence.effects.EffectError
import fluence.log.Log
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.{ReceiptBus, TxProcessor}
import fluence.statemachine.api.data.BlockReceipt
import shapeless._

import scala.language.higherKinds

class EmbeddedBlockProducer[F[_]: Monad](
  txProcessor: TxProcessor[F],
  receiptBus: ReceiptBus[F],
  lastHeight: Ref[F, Long],
  blocksQueue: fs2.concurrent.Queue[F, Long]
) extends BlockProducer[F] {
  override type Block = Long

  /**
   * Retrieve the last height, known locally
   *
   */
  override def lastKnownHeight()(implicit log: Log[F]): F[Long] =
    lastHeight.get

  /**
   * Stream of blocks, starting with the given height
   *
   * @param fromHeight All newer blocks shall appear in the stream
   * @return Stream of blocks
   */
  override def blockStream(fromHeight: Long)(implicit log: Log[F]): fs2.Stream[F, Block] =
    blocksQueue.dequeue.filter(_ >= fromHeight)

  /**
   * Send (asynchronously) a transaction to the block producer, so that it should later get into a block
   * TODO is it thread safe?
   *
   * @param txData Transaction data
   */
  override def sendTx(txData: Array[Byte])(implicit log: Log[F]): EitherT[F, EffectError, TxResponse] =
    txProcessor
      .processTx(txData)
      .flatTap(
        _ ⇒
          for {
            sh ← txProcessor.commit()
            bv ← receiptBus.getVmHash(sh.height)

            // Here we "do" block uploading, and then provide BlockReceipt back via ReceiptBus
            // receipt <- upload(block)
            receipt = BlockReceipt(sh.height, sh.hash ++ bv)
            _ ← receiptBus.sendBlockReceipt(receipt)

            _ ← EitherT.right(lastHeight.set(sh.height))
            _ ← EitherT.right(blocksQueue.enqueue1(sh.height))
          } yield ()
      )

}

object EmbeddedBlockProducer {

  def apply[F[_]: Concurrent, C <: HList](
    machine: StateMachine.Aux[F, C]
  )(
    implicit txp: ops.hlist.Selector[C, TxProcessor[F]],
    rb: ops.hlist.Selector[C, ReceiptBus[F]]
  ): F[BlockProducer.Aux[F, Long]] =
    for {
      lastHeight ← Ref.of[F, Long](0)
      blockQueue ← fs2.concurrent.Queue.circularBuffer[F, Long](16)
    } yield new EmbeddedBlockProducer[F](
      machine.command[TxProcessor[F]],
      machine.command[ReceiptBus[F]],
      lastHeight,
      blockQueue
    ) {
      override type Block = Long
    }

}
