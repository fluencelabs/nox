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

import java.nio.file.Path

import cats.effect.{ConcurrentEffect, ContextShift, Resource, Timer}
import fluence.effects.sttp.SttpEffect
import fluence.effects.tendermint.block.history.db.Blockstore
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.tendermint.rpc.websocket.TendermintWebsocketRpc
import fluence.log.Log

import scala.language.higherKinds

class Tendermint[F[_]](
  val rpc: TendermintHttpRpc[F],
  val wrpc: TendermintWebsocketRpc[F],
  val blockstore: Blockstore[F]
)

object Tendermint {
  def apply[F[_]: ConcurrentEffect: Timer: SttpEffect: ContextShift: Log](
                                                                           host: String,
                                                                           port: Short,
                                                                           path: Path
                                                                         ): Resource[F, Tendermint[F]] =
      Blockstore.make[F](path).map{bs ⇒
        val rpc = TendermintHttpRpc[F](host, port)
        new Tendermint[F](
          rpc,
          TendermintWebsocketRpc(host, port, rpc, bs),
          bs
        )
      }

}
