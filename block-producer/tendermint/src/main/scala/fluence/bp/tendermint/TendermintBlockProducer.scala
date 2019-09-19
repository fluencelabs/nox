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

import cats.data.EitherT
import fluence.bp.api.BlockProducer
import fluence.bp.tx.TxResponse
import fluence.effects.EffectError
import fluence.log.Log

import scala.language.higherKinds

class TendermintBlockProducer[F[_]] extends BlockProducer[F] {
  // TODO put the real Tendermint block here
  override type Block = this.type

  /**
   * Retrieve the last height, known locally
   * TODO read it from BlockStore?
   */
  override def lastKnownHeight()(implicit log: Log[F]): F[Long] = ???

  /**
   * Stream of blocks, starting with the given height
   * TODO get it from BlockStore, then switch to websocket?
   *
   * @param fromHeight All newer blocks shall appear in the stream
   * @return Stream of blocks
   */
  override def blockStream(fromHeight: Long)(implicit log: Log[F]): fs2.Stream[F, Block] = ???

  /**
   * Send (asynchronously) a transaction to the block producer, so that it should later get into a block
   * TODO really sendTx via Tendermint RPC
   *
   * @param txData Transaction data
   */
  override def sendTx(txData: Array[Byte])(implicit log: Log[F]): EitherT[F, EffectError, TxResponse] = ???
}
