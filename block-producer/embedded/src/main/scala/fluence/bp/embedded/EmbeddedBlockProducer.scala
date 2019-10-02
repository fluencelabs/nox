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
import fluence.bp.api.{BlockProducer, BlockProducerStatus, BlockStream}
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
  blocksTopic: fs2.concurrent.Topic[F, Option[SimpleBlock]]
) extends BlockProducer[F] {

  /**
   * Product (HList) of all types to access Command side of this block producer.
   */
  override type Commands = HNil

  /**
   * Implementations for the command side
   */
  override protected val commands: Commands = HNil

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
            _ <- EitherT.right(blocksTopic.publish1(Some(SimpleBlock(sh.height, Seq(ByteVector(txData))))))
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
  ): F[BlockProducer.Aux[F, BlockStream[F, SimpleBlock] :: HNil]] =
    for {
      blocksTopic <- fs2.concurrent.Topic[F, Option[SimpleBlock]](None)
    } yield new EmbeddedBlockProducer[F](machine.command[TxProcessor[F]], blocksTopic)
      .extend(new BlockStream[F, SimpleBlock] {
        override def freshBlocks(implicit log: Log[F]): fs2.Stream[F, SimpleBlock] =
          blocksTopic.subscribe(1).unNone

        override def blocksSince(height: Long)(implicit log: Log[F]): fs2.Stream[F, SimpleBlock] =
          blocksTopic.subscribe(Int.MaxValue).unNone.filter(_.height >= height)
      })

}
