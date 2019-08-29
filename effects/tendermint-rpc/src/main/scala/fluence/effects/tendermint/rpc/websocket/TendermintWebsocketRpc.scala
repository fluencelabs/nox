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

import cats.Monad
import cats.effect._
import cats.syntax.compose._
import cats.syntax.flatMap._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.history.db.Blockstore
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fs2.concurrent.Queue

import scala.language.higherKinds

/**
 * Algebra for Tendermint's websocket RPC
 */
trait TendermintWebsocketRpc[F[_]] {

  /**
   * Config for the underlying websocket
   */
  val websocketConfig: WebsocketConfig

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
    event: String
  )(implicit log: Log[F], backoff: Backoff[EffectError]): Resource[F, Queue[F, Event]]
}

object TendermintWebsocketRpc {

  /**
   * Creates Tendermint Websocket RPC
   *
   * @param host Host to query status from
   * @param port Port to query status from
   * @param websocketConfig Config for the websocket connection to Tendermint
   * @tparam F Concurrent effect
   * @return Tendermint websocket RPC instance. Note that it should be stopped at some point, and can't be used after it's stopped
   */
  def make[F[_]: ConcurrentEffect: Timer: Monad: ContextShift](
    host: String,
    port: Int,
    httpRpc: TendermintHttpRpc[F],
    blockstore: Blockstore[F],
    websocketConfig: WebsocketConfig = WebsocketConfig(),
  ): TendermintWebsocketRpc[F] = new TendermintWebsocketRpcImpl[F](host, port, httpRpc, blockstore, websocketConfig)
}
