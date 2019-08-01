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
import cats.effect.{Concurrent, Resource, Timer}
import cats.effect.concurrent.{Deferred, Ref}
import cats.syntax.functor._
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.log.Log
import fluence.statemachine.data.Tx

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

  def apply[F[_]: Log: Concurrent: Timer, G[_]](
    tendermint: TendermintRpc[F],
    appId: Long,
    maxBlocksTries: Int = 3
  )(
    implicit P: Parallel[F, G]
  ): F[ResponseSubscriber[F]] =
    Ref
      .of[F, Map[Tx.Head, ResponsePromise[F]]](
        Map.empty[Tx.Head, ResponsePromise[F]]
      )
      .map(r => new ResponseSubscriberImpl(r, tendermint, appId, maxBlocksTries))

  def make[F[_]: Log: Concurrent: Timer, G[_]](
    tendermint: TendermintRpc[F],
    appId: Long,
    maxBlocksTries: Int = 3
  )(
    implicit P: Parallel[F, G]
  ): Resource[F, ResponseSubscriber[F]] = Resource.liftF(apply(tendermint, appId, maxBlocksTries))
}
