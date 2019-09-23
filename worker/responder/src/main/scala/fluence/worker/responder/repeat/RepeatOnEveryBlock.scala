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

package fluence.worker.responder.repeat

import cats.effect.{Concurrent, Resource, Timer}
import fluence.bp.api.BlockProducer
import fluence.bp.tx.Tx
import fluence.effects.resources.MakeResource
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.worker.responder.SendAndWait
import fluence.worker.responder.resp.{AwaitedResponse, TxAwaitError}

import scala.language.higherKinds

/**
 * Service to call subscribed transactions after each tendermint block.
 *
 */
trait RepeatOnEveryBlock[F[_]] {

  /**
   * Makes a subscription by transaction.
   * The master node will send a transaction to state machine after every block
   * and will return response to a connected client.
   *
   * @param data a transaction
   * @return a stream of responses every block
   */
  def subscribe(subscriptionKey: SubscriptionKey, data: Tx.Data)(
    implicit log: Log[F]
  ): F[fs2.Stream[F, Either[TxAwaitError, AwaitedResponse]]]

  def unsubscribe(subscriptionKey: SubscriptionKey)(implicit log: Log[F]): F[Boolean]
}

object RepeatOnEveryBlock {

  /**
   *
   * @param producer websocket service to subscribe for a new blocks
   * @param sendAndWait for transaction execution
   */
  def make[F[_]: Timer: Concurrent: Log](
    producer: BlockProducer[F],
    sendAndWait: SendAndWait[F]
  )(implicit backoff: Backoff[EffectError]): Resource[F, RepeatOnEveryBlock[F]] =
    for {
      subs <- MakeResource.refOf[F, Map[String, Subscription[F]]](Map.empty)
      instance = new RepeatOnEveryBlockImpl[F](subs, producer, sendAndWait)
      _ ← instance.start()
    } yield instance
}
