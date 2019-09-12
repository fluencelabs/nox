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

import cats.effect.{Concurrent, Resource, Timer}
import cats.effect.concurrent.Ref
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.traverse._
import cats.syntax.applicative._
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.tendermint.rpc.websocket.TendermintWebsocketRpc
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.node.MakeResource
import fluence.node.workers.api.websocket.WorkerWebsocket.SubscriptionKey
import fluence.node.workers.subscription.PerBlockTxExecutor.{
  Event,
  Init,
  Quit,
  Response,
  Subscription,
  TendermintResponse
}
import fluence.statemachine.api.tx.Tx
import fs2.concurrent.{SignallingRef, Topic}

import scala.language.higherKinds
import scala.util.Random

class PerBlockTxExecutorImpl[F[_]: Timer: Concurrent](
  subscriptions: Ref[F, Map[String, Subscription[F]]],
  tendermintWRpc: TendermintWebsocketRpc[F],
  tendermintRpc: TendermintHttpRpc[F],
  waitResponseService: WaitResponseService[F]
)(
  implicit backoff: Backoff[EffectError] = Backoff.default[EffectError]
) extends PerBlockTxExecutor[F] {

  /**
   * Makes a subscription by transaction.
   * The master node will send a transaction to state machine after every block
   * and will return response to a connected client.
   *
   * @param data a transaction
   * @return a stream of responses every block
   */
  override def subscribe(key: SubscriptionKey, data: Tx.Data)(
    implicit log: Log[F]
  ): F[fs2.Stream[F, TendermintResponse]] =
    for {
      _ <- log.debug(s"Subscribe for id: ${key.subscriptionId}, txHash: ${key.txHash}")
      topic <- Topic[F, Event](Init)
      signal <- SignallingRef[F, Boolean](false)
      subState <- subscriptions.modify { subs =>
        subs.get(key.txHash) match {
          case Some(sub) => (subs.updated(key.txHash, sub.copy(subNumber = sub.subNumber + 1)), sub)
          case None =>
            val newState = Subscription(data, topic, 1)
            (subs + (key.txHash -> newState), newState)
        }
      }
    } yield {
      subState.topic
        .subscribe(10)
        .evalMap {
          case q @ Quit(id) if id == key.subscriptionId => signal.set(true) as (q: Event)
          case v                                        => v.pure[F]
        }
        .collect {
          case Response(v) => v
        }
        .interruptWhen(signal)
    }

  override def unsubscribe(key: SubscriptionKey)(implicit log: Log[F]): F[Boolean] =
    for {
      _ <- log.debug(s"Unsubscribe for id: ${key.subscriptionId}, txHash: $key")
      (isOk, topicToCloseSubscription) <- subscriptions.modify { subs =>
        subs.get(key.txHash) match {
          case Some(sub) =>
            val updated =
              if (sub.subNumber == 1) subs - key.txHash
              else subs.updated(key.txHash, sub.copy(subNumber = sub.subNumber - 1))
            (updated, (true, Option(sub.topic)))
          case None => (subs, (false, None))
        }
      }
      _ <- topicToCloseSubscription match {
        case Some(q) => q.publish1(Quit(key.subscriptionId))
        case None    => ().pure[F]
      }
    } yield isOk

  /**
   * Starts a background process to execute subscribed transactions for a worker,
   * polls service for a new response after each block.
   *
   */
  def start()(implicit log: Log[F]): Resource[F, Unit] =
    log.scope("startBlockTxExecutor") { implicit log =>
      for {
        lastHeight <- Resource.liftF(
          backoff.retry(tendermintRpc.consensusHeight(), e => log.error("retrieving consensus height", e))
        )
        _ <- Log.resource.info("Creating subscription for tendermint blocks")
        blockStream = tendermintWRpc.subscribeNewBlock(lastHeight)
        pollingStream = blockStream
          .evalTap(b => log.debug(s"got block ${b.header.height}"))
          .evalMap(_ => processSubscriptions())
        _ <- MakeResource.concurrentStream(pollingStream)
      } yield ()
    }

  /**
   * Generates unique header for transaction and call sentTxAwaitResponse
   *
   */
  private def waitTx(key: String, data: Tx.Data)(
    implicit log: Log[F]
  ): F[Either[TxAwaitError, TendermintQueryResponse]] = {
    val randomStr = Random.alphanumeric.take(8).mkString
    val head = Tx.Head(s"pubsub-$key-$randomStr", 0)
    val tx = Tx(head, data)

    waitResponseService.sendTxAwaitResponse(tx.generateTx(), None)
  }

  private def processSubscriptions()(implicit log: Log[F]) = {
    import cats.instances.list._
    for {
      subs <- subscriptions.get
      _ <- log.debug(s"Processing ${subs.size} subscriptions")
      tasks = subs.map {
        case (key, Subscription(data, topic, _)) =>
          for {
            response <- waitTx(key, data)
            _ <- log.trace(s"Publishing $response for $key")
            _ <- topic.publish1(Response(response))
          } yield {}
      }
      _ <- tasks.toList.traverse(Concurrent[F].start)
    } yield ()
  }
}
