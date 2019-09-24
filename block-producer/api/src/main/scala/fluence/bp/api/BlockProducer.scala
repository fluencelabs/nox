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

package fluence.bp.api

import cats.data.EitherT
import fluence.bp.tx.TxResponse
import fluence.log.Log
import fluence.effects.EffectError
import shapeless._

import scala.language.higherKinds

trait BlockProducer[F[_]] {
  self ⇒
  type Block

  /**
   * Product (HList) of all types to access Command side of this block producer.
   */
  type Commands <: HList

  /**
   * Implementations for the command side
   */
  protected val commands: Commands

  /**
   * Access for a particular command. Usage: `producer.command[ConcreteCommandType]`
   *
   * @param cmd Selector; always available implicitly
   * @tparam C Command's type
   */
  final def command[C](implicit cmd: ops.hlist.Selector[Commands, C]): C = cmd(commands)

  /**
   * Extend this StateMachine with one more command-side service
   *
   * @param cmd Command-side service
   * @tparam T Service's type
   * @return Extended StateMachine
   */
  final def extend[T](cmd: T): BlockProducer.Aux[F, Block, T :: Commands] = new BlockProducer[F] {
    override type Block = self.Block

    override type Commands = T :: self.Commands

    override protected val commands: Commands = cmd :: self.commands

    override def blockStream(fromHeight: Option[Long])(implicit log: Log[F]): fs2.Stream[F, Block] =
      self.blockStream(fromHeight)

    override def sendTx(txData: Array[Byte])(implicit log: Log[F]): EitherT[F, EffectError, TxResponse] =
      self.sendTx(txData)

    override def status()(implicit log: Log[F]): EitherT[F, EffectError, BlockProducerStatus] =
      self.status()
  }

  /**
   * Stream of blocks, starting with the given height
   *
   * @param fromHeight If defined, all blocks of greater height shall appear in the stream.
   *                   If empty, stream shall start from current block.
   * @return Stream of blocks
   */
  def blockStream(fromHeight: Option[Long])(implicit log: Log[F]): fs2.Stream[F, Block]

  /**
   * Send (asynchronously) a transaction to the block producer, so that it should later get into a block
   *
   * @param txData Transaction data
   */
  def sendTx(txData: Array[Byte])(implicit log: Log[F]): EitherT[F, EffectError, TxResponse]

  /**
   * Provides current status of BlockProducer
   */
  def status()(implicit log: Log[F]): EitherT[F, EffectError, BlockProducerStatus]
}

object BlockProducer {

  type Aux[F[_], B, C <: HList] = BlockProducer[F] {
    type Block = B
    type Commands = C
  }

  type AuxC[F[_], C <: HList] = BlockProducer[F] {
    type Commands = C
  }

  type AuxB[F[_], B] = BlockProducer[F] {
    type Block = B
  }
}
