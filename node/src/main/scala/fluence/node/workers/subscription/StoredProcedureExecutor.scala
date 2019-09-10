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

import cats.Monad
import cats.effect.concurrent.Ref
import cats.effect.{Concurrent, Resource, Sync, Timer}
import fluence.crypto.Crypto.Hasher
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.tendermint.rpc.websocket.TendermintWebsocketRpc
import fluence.statemachine.data.Tx
import cats.syntax.functor._
import fluence.log.Log
import fluence.node.workers.api.websocket.WorkerWebsocket.SubscriptionKey
import fluence.node.workers.subscription.StoredProcedureExecutor.{Event, TendermintResponse}
import fs2.concurrent.Topic

import scala.language.higherKinds

/**
 * Service to call subscribed transactions after each tendermint block.
 *
 */
trait StoredProcedureExecutor[F[_]] {

  /**
   * Makes a subscription by transaction.
   * The master node will send a transaction to state machine after every block
   * and will return response to a connected client.
   *
   * @param data a transaction
   * @return a stream of responses every block
   */
  def subscribe(subscriptionKey: SubscriptionKey, data: Tx.Data): F[fs2.Stream[F, TendermintResponse]]

  def unsubscribe(subscriptionKey: SubscriptionKey): F[Boolean]

  /**
   * Gets all transaction subscribes for appId and trying to poll service for new responses.
   *
   */
  def start(): Resource[F, Unit]
}

object StoredProcedureExecutor {

  /**
   * Data about subscription.
   *
   * @param tx transaction that will be processed after each block
   * @param topic for publishing events to subscribers
   * @param subNumber number of subscriptions, the subscription should be deleted after subNumber become zero
   */
  private[subscription] case class Subscription[F[_]](
    tx: Tx.Data,
    topic: Topic[F, Event],
    subNumber: Int
  )

  sealed trait Event
  case class Response(value: TendermintResponse) extends Event
  case object Init extends Event
  case class Quit(id: String) extends Event

  type TendermintResponse = Either[TxAwaitError, TendermintQueryResponse]

  def make[F[_]: Monad: Timer: Sync: Concurrent: Log](
    tendermintWRpc: TendermintWebsocketRpc[F],
    tendermintRpc: TendermintHttpRpc[F],
    waitResponseService: WaitResponseService[F],
    hasher: Hasher[Array[Byte], String]
  ): Resource[F, StoredProcedureExecutor[F]] =
    for {
      subs <- Resource.liftF(Ref.of[F, Map[String, Subscription[F]]](Map.empty))
    } yield new StoredProcedureExecutorImpl[F](subs, tendermintWRpc, tendermintRpc, waitResponseService, hasher)
}
