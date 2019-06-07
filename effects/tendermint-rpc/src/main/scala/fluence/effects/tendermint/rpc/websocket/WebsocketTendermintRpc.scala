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

package fluence.effects.tendermint.rpc.websocket

import cats.effect._
import cats.syntax.compose._
import cats.syntax.flatMap._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fs2.concurrent.Queue

import scala.language.higherKinds

/**
 * Algebra for Tendermint's websocket RPC
 */
trait WebsocketTendermintRpc[F[_]] {

  /**
   * Subscribe on new blocks from Tendermint, retrieves missing blocks and keeps them in order
   *
   * @param lastKnownHeight Height of the block that was already processed (uploaded, and its receipt stored)
   * @return Stream of blocks, strictly in order, without any repetitions
   */
  def subscribeNewBlock(
    lastKnownHeight: Long
  )(implicit log: Log[F], backoff: Backoff[EffectError] = Backoff.default): fs2.Stream[F, Block]

  protected def subscribe(
    event: String,
  )(implicit backoff: Backoff[EffectError]): Resource[F, Queue[F, Event]]
}
