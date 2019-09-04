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

import cats.effect.Resource
import fluence.effects.tendermint.block.data.Block
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fs2.concurrent.Queue

import scala.language.higherKinds

trait TestTendermintWebsocketRpc[F[_]] extends TendermintWebsocketRpc[F] with TestTendermintHttpRpc[F] {

  override val websocketConfig: WebsocketConfig = throw new NotImplementedError("def websocketConfig")

  def subscribeNewBlock(
    lastKnownHeight: Long
  )(implicit log: Log[F], backoff: Backoff[EffectError]): fs2.Stream[F, Block] =
    throw new NotImplementedError("def subscribeNewBlock")

  protected def subscribe(
    event: String
  )(implicit log: Log[F], backoff: Backoff[EffectError]): Resource[F, Queue[F, Event]] =
    throw new NotImplementedError("def subscribe")
}
