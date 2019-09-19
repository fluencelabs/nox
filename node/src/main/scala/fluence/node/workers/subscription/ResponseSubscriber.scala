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

package fluence.node.workers.subscription

import cats.Parallel
import cats.effect.concurrent.{Deferred, Ref}
import cats.effect.{Concurrent, Resource, Timer}
import cats.syntax.functor._
import fluence.bp.tx.Tx
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.tendermint.rpc.websocket.TendermintWebsocketRpc
import fluence.log.Log

import scala.language.higherKinds

/**
 * Interface that describes how a client can subscribe for responses from tendermint.
 */
trait ResponseSubscriber[F[_]] {

  /**
   * Makes a subscription by transaction id.
   * The master node will query state machine for response by this subscription each generated block.
   *
   * @param id transaction id: sessionId/nonce
   * @return a promise that will be completed after response will be received
   */
  def subscribe(id: Tx.Head): F[Deferred[F, TendermintQueryResponse]]

  /**
   * Gets all request subscribes for appId and trying to poll service for responses.
   *
   */
  def start(): Resource[F, Unit]
}

object ResponseSubscriber {
  // TODO: move to config
  val MaxBlockTries = 10

  val PubSubSessionPrefix = "pubsub"

  def make[F[_]: Log: Concurrent: Timer: Parallel](
    tendermintRpc: TendermintHttpRpc[F],
    tendermintWRpc: TendermintWebsocketRpc[F],
    appId: Long,
    maxBlocksTries: Int = MaxBlockTries
  ): Resource[F, ResponseSubscriber[F]] = Resource.liftF(
    Ref
      .of[F, Map[Tx.Head, ResponsePromise[F]]](Map.empty)
      .map(r => new ResponseSubscriberImpl(r, tendermintRpc, tendermintWRpc, appId, maxBlocksTries))
  )
}
