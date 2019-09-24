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
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.bp.api.{BlockProducer, BlockProducerStatus}
import fluence.bp.tx.TxResponse
import fluence.effects.EffectError
import fluence.log.Log
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command.TxProcessor
import scodec.bits.ByteVector
import shapeless._

import scala.language.higherKinds

class EmbeddedBlockProducer[F[_]: Monad](
  txProcessor: TxProcessor[F],
  blocksQueue: fs2.concurrent.Queue[F, SimpleBlock]
) extends BlockProducer[F] {
  override type Block = SimpleBlock

  /**
   * Product (HList) of all types to access Command side of this block producer.
   */
  override type Commands = HNil

  /**
   * Implementations for the command side
   */
  override protected val commands: Commands = HNil

  /**
   * Stream of blocks, starting with the given height
   *
   * @param fromHeight All newer blocks shall appear in the stream
   * @return Stream of blocks
   */
  override def blockStream(fromHeight: Option[Long])(implicit log: Log[F]): fs2.Stream[F, Block] =
    fromHeight.foldLeft(blocksQueue.dequeue) { case (s, h) => s.filter(_.height >= h) }

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

            _ ← EitherT.right(blocksQueue.enqueue1(SimpleBlock(sh.height, Seq(ByteVector(txData)))))
          } yield ()
      )

  /**
   * Provides current status of BlockProducer
   */
  override def status()(implicit log: Log[F]): EitherT[F, EffectError, BlockProducerStatus] =
    EitherT.pure(BlockProducerStatus("Embedded block producer operating normally"))
}

object EmbeddedBlockProducer {

  def apply[F[_]: Concurrent, C <: HList](
    machine: StateMachine.Aux[F, C]
  )(
    implicit txp: ops.hlist.Selector[C, TxProcessor[F]]
  ): F[BlockProducer.Aux[F, SimpleBlock, HNil]] =
    for {
      blockQueue ← fs2.concurrent.Queue.circularBuffer[F, SimpleBlock](16)
    } yield new EmbeddedBlockProducer[F](machine.command[TxProcessor[F]], blockQueue)

}
