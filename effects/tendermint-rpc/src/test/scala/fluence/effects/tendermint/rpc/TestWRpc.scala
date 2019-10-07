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

import cats.Monad
import cats.effect.concurrent.Deferred
import cats.effect.{Concurrent, ConcurrentEffect, ContextShift, Timer}
import cats.syntax.either._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.websocket.{TendermintWebsocketRpcImpl, WebsocketConfig}
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log

import scala.language.higherKinds

class TestWRpc[F[_]: ConcurrentEffect: Timer: Monad: ContextShift](host: String, port: Int)
    extends TendermintWebsocketRpcImpl[F](host, port, new TestHttpRpc[F], DisabledBlockstore(), WebsocketConfig()) {

  override val websocketConfig: WebsocketConfig = WebsocketConfig()

  /**
   * Subscribe on new blocks from Tendermint, retrieves missing blocks and keeps them in order
   *
   * @param lastKnownHeight Height of the block that was already processed (uploaded, and its receipt stored)
   * @return Stream of blocks, strictly in order, without any repetitions
   */
  override def subscribeNewBlock(
    lastKnownHeight: Option[Long]
  )(implicit log: Log[F], backoff: Backoff[EffectError]): fs2.Stream[F, Block] = {
    import cats.syntax.flatMap._
    import cats.syntax.functor._

    import scala.concurrent.duration._

    val timeout = 5.seconds

    val fiberF =
      for {
        promise <- Deferred[F, Unit]
        fiber <- Concurrent[F].start(
          Timer[F].sleep(timeout) >>
            Log[F].error(s"subscribeNewBlock timed out after $timeout") >>
            promise.complete(())
        )
      } yield (fiber, promise)

    fs2.Stream.eval(fiberF).flatMap {
      case (fiber, promise) =>
        val signal = promise.get.map(_ => ().asRight[Throwable])
        super
          .subscribeNewBlock(lastKnownHeight)
          .evalTap(_ => fiber.cancel)
          .interruptWhen(signal)
    }
  }
}
