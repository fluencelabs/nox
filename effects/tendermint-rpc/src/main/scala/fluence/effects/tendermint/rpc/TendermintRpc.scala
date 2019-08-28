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

package fluence.effects.tendermint.rpc

import cats.effect.{ConcurrentEffect, ContextShift, Resource, Timer}
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.rpc.http.{TendermintHttpRpc, TendermintHttpRpcImpl}
import fluence.effects.tendermint.rpc.websocket.{TendermintWebsocketRpc, WebsocketConfig}
import fluence.log.Log

import scala.language.higherKinds

trait TendermintRpc[F[_]] extends TendermintHttpRpc[F] with TendermintWebsocketRpc[F]

object TendermintRpc {

  /**
   * Runs a WorkerRpc with F effect, acquiring some resources for it
   *
   * @param hostName Hostname to query status from
   * @param port Port to query status from
   * @param websocketConfig Config for the websocket connection to Tendermint
   * @tparam F Concurrent effect
   * @return Worker RPC instance. Note that it should be stopped at some point, and can't be used after it's stopped
   */
  def make[F[_]: ConcurrentEffect: Timer: SttpEffect: Log: ContextShift](
    hostName: String,
    port: Short,
    websocketConfig: WebsocketConfig = WebsocketConfig()
  ): Resource[F, TendermintRpc[F]] =
    Resource.pure(
      new TendermintHttpRpcImpl[F](hostName, port, websocketConfig)
    )
}
