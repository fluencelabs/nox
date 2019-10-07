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

package fluence.bp.tendermint

import cats.Monad
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.data.EitherT
import fluence.bp.api.{BlockProducer, BlockProducerStatus, BlockStream, DialPeers}
import fluence.bp.tx.TxResponse
import fluence.effects.EffectError
import fluence.effects.tendermint.block.data
import fluence.effects.tendermint.block.data.Block
import fluence.log.Log
import shapeless._

import scala.language.higherKinds

class TendermintBlockProducer[F[_]: Monad](
  tendermint: Tendermint[F],
  checkStatus: EitherT[F, EffectError, _]
) extends BlockProducer[F] {

  /**
   * Product (HList) of all types to access Command side of this block producer.
   */
  override type Commands = HNil

  /**
   * Implementations for the command side
   */
  override protected val commands: HNil = HNil

  /**
   * Send (asynchronously) a transaction to the block producer, so that it should later get into a block
   *
   * @param txData Transaction data
   */
  override def sendTx(txData: Array[Byte])(implicit log: Log[F]): EitherT[F, EffectError, TxResponse] =
    tendermint.rpc.broadcastTxSync(txData).leftMap(identity[EffectError])

  /**
   * Provides current status of BlockProducer
   * TODO provide more granular status info
   */
  override def status()(implicit log: Log[F]): EitherT[F, EffectError, BlockProducerStatus] =
    checkStatus >>
      tendermint.rpc.statusParsed
        .map(
          st ⇒ BlockProducerStatus(s"Tendermint latest block height: ${st.sync_info.latest_block_height}")
        )
        .leftMap(identity[EffectError])
}

object TendermintBlockProducer {

  def apply[F[_]: Monad](
    tendermint: Tendermint[F],
    checkStatus: EitherT[F, EffectError, _]
  ): BlockProducer.Aux[F, BlockStream[F, data.Block] :: DialPeers[F] :: HNil] =
    new TendermintBlockProducer[F](tendermint, checkStatus)
      .extend[DialPeers[F]](new DialPeers[F] {
        override def dialPeers(peers: Seq[String])(implicit log: Log[F]): EitherT[F, EffectError, Unit] =
          tendermint.rpc
            .unsafeDialPeers(peers, persistent = true)
            .leftMap(identity[EffectError])
            .flatTap(s ⇒ Log.eitherT[F, EffectError].debug(s"Tendermint unsafeDialPeers replied: $s"))
            .void
      })
      .extend[BlockStream[F, data.Block]](
        new BlockStream[F, data.Block] {
          override def freshBlocks(implicit log: Log[F]): fs2.Stream[F, Block] =
            tendermint.wrpc.subscribeNewBlock(None)

          override def blocksSince(height: Long)(implicit log: Log[F]): fs2.Stream[F, Block] =
            tendermint.wrpc.subscribeNewBlock(Some(height))
        }
      )
}
