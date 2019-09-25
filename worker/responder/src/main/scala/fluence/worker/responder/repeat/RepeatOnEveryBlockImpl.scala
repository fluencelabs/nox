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

import cats.data.EitherT
import cats.effect.concurrent.Ref
import cats.effect.{Concurrent, Resource, Sync, Timer}
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.traverse._
import fluence.bp.api.BlockProducer
import fluence.bp.tx.{Tx, TxsBlock}
import fluence.effects.resources.MakeResource
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.worker.responder.resp.{AwaitedResponse, TxAwaitError}
import fluence.worker.responder.{AwaitResponses, SendAndWait}
import fs2.concurrent.{SignallingRef, Topic}
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.Random

private[repeat] class RepeatOnEveryBlockImpl[F[_]: Timer: Concurrent, B: TxsBlock](
  subscriptions: Ref[F, Map[String, Subscription[F]]],
  producer: BlockProducer.AuxB[F, B],
  waitResponseService: SendAndWait[F]
)(
  implicit backoff: Backoff[EffectError]
) extends RepeatOnEveryBlock[F] {

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
  ): F[fs2.Stream[F, AwaitedResponse.OrError]] =
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
        _ <- Log.resource.info("Creating subscription for tendermint blocks")
        blockStream = producer.blockStream(fromHeight = None)
        pollingStream = blockStream
          .evalTap(
            b =>
              log.debug(
                s"got block ${TxsBlock[B]
                  .height(b)} nonEmptyBlock: ${nonEmptyBlock(b)} ${TxsBlock[B].txs(b).map(_.takeWhile(_ != '\n'.toByte).decodeUtf8).mkString(", ")}"
              )
          )
          .filter(nonEmptyBlock)
          .evalMap(_ => processSubscriptions())
        _ <- MakeResource.concurrentStream(pollingStream)
      } yield ()
    }

  /**
   * @return true if block contains any non-pubsub txs, false otherwise
   */
  private def nonEmptyBlock(block: B): Boolean = TxsBlock[B].txs(block).exists(!isRepeatTx(_))

  private def isRepeatTx(tx: ByteVector) = tx.startsWith(RepeatOnEveryBlockImpl.SessionPrefixBytes)

  /**
   * Generates unique header for transaction and call sentTxAwaitResponse
   *
   */
  private def waitTx(key: String, data: Tx.Data)(
    implicit log: Log[F]
  ): EitherT[F, TxAwaitError, AwaitedResponse] =
    for {
      sid <- EitherT.right(sessionId(key))
      tx = Tx(Tx.Head(sid, 0), data)
      response <- waitResponseService.sendTxAwaitResponse(tx.generateTx())
    } yield response

  private def sessionId(key: String): F[String] =
    Sync[F]
      .delay(Random.alphanumeric.take(8).mkString)
      .map(
        rnd => s"${RepeatOnEveryBlockImpl.SessionPrefix}-$key-$rnd"
      )

  private def processSubscriptions()(implicit log: Log[F]) = {
    import cats.instances.list._
    for {
      subs <- subscriptions.get
      _ <- log.debug(s"Processing ${subs.size} subscriptions")
      tasks = subs.map {
        case (key, Subscription(data, topic, _)) =>
          for {
            response <- waitTx(key, data).value
            _ <- log.trace(s"Publishing $response for $key")
            _ <- topic.publish1(Response(response))
          } yield ()
      }
      _ <- tasks.toList.traverse(Concurrent[F].start)
    } yield ()
  }
}

object RepeatOnEveryBlockImpl {
  private val SessionPrefix = "repeat"
  private val SessionPrefixBytes = ByteVector(SessionPrefix.getBytes)
}
