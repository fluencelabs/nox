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

package fluence.node.workers.api.websocket

import cats.data.EitherT
import cats.effect.Concurrent
import fluence.log.Log
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import fluence.node.workers.api.WorkerApi
import fluence.node.workers.api.websocket.WebsocketResponses.WebsocketResponse
import fluence.node.workers.api.websocket.WorkerWebsocket.{Subscription, SubscriptionKey}
import fluence.node.workers.subscription.PerBlockTxExecutor.TendermintResponse
import fluence.node.workers.subscription.{OkResponse, PendingResponse, RpcErrorResponse, TimedOutResponse}
import fs2.concurrent.{NoneTerminatedQueue, Queue}
import io.circe
import io.circe.parser.parse
import io.circe.syntax._
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * The layer between messages from Websocket and WorkerAPI.
 *
 * @param subscriptionsStorage keeps all subscriptions of websocket connection
 * @param outputQueue for sending messages into websocket
 */
class WorkerWebsocket[F[_]: Concurrent](
  workerApi: WorkerApi[F],
  subscriptionsStorage: SubscriptionStorage[F],
  outputQueue: NoneTerminatedQueue[F, WebsocketResponse]
)(implicit log: Log[F]) {
  import WebsocketRequests._
  import WebsocketResponses._

  /**
   * A stream with events of all subscriptions.
   *
   */
  val subscriptionEventStream: fs2.Stream[F, String] = outputQueue.dequeue.map(_.asJson.noSpaces)

  /**
   * Unsubscribes from all subscriptions and closes the queue.
   *
   */
  def closeWebsocket(): F[Unit] = {
    import cats.syntax.traverse._
    import cats.instances.list._
    for {
      subs <- subscriptionsStorage.getSubscriptions
      _ <- subs.keys.toList.traverse(workerApi.unsubscribe)
      _ <- outputQueue.enqueue1(None)
    } yield ()
  }

  /**
   * Parse input and call WebsocketAPI.
   * @param input messages from websocket
   */
  def processRequest(input: String): F[String] = {
    val result = for {
      request <- EitherT
        .fromEither(parse(input).flatMap(_.as[WebsocketRequest]))
      _ <- Log.eitherT[F, circe.Error].trace("Processing input: " + input)
      response <- EitherT.liftF[F, io.circe.Error, WebsocketResponse](callApi(request))
      _ <- Log.eitherT[F, circe.Error].trace("Response: " + response)
    } yield response
    result.value.map {
      case Right(v)    => v
      case Left(error) => ErrorResponse("", s"Cannot parse msg. Error: $error, msg: $input")
    }.map(_.asJson.noSpaces)
  }

  private def toWebsocketResponse(requestId: String, response: TendermintResponse): WebsocketResponse = {
    response match {
      case Right(OkResponse(_, response))    => TxWaitResponse(requestId, response)
      case Right(RpcErrorResponse(_, error)) => ErrorResponse(requestId, error.getMessage)
      case Right(TimedOutResponse(_, tries)) =>
        ErrorResponse(requestId, s"Cannot get response after $tries generated blocks")
      case Right(PendingResponse(_)) => ErrorResponse(requestId, s"Unexpected error.")
      case Left(error)               => ErrorResponse(requestId, error.msg)
    }
  }

  private def callApi(input: WebsocketRequest): F[WebsocketResponse] =
    input match {
      case TxRequest(tx, id, requestId) =>
        workerApi.sendTx(tx, id).map {
          case Right(data) => TxResponse(requestId, data)
          case Left(error) => ErrorResponse(requestId, error.getMessage)
        }
      case QueryRequest(path, data, id, requestId) =>
        workerApi.query(data, path, id).map {
          case Right(data) => QueryResponse(requestId, data)
          case Left(error) => ErrorResponse(requestId, error.getMessage)
        }
      case TxWaitRequest(tx, id, requestId) =>
        workerApi
          .sendTxAwaitResponse(tx, id)
          .map(toWebsocketResponse(requestId, _))
      case LastManifestRequest(requestId) =>
        workerApi.lastManifest().map(block => LastManifestResponse(requestId, block.map(_.jsonString)))
      case StatusRequest(requestId) =>
        workerApi.tendermintStatus().map {
          case Right(status) => StatusResponse(requestId, status)
          case Left(error)   => ErrorResponse(requestId, error.getMessage)
        }
      case P2pPortRequest(requestId) => workerApi.p2pPort().map(port => P2pPortResponse(requestId, port))
      case SubscribeRequest(requestId, subscriptionId, tx) =>
        val key = SubscriptionKey.generate(subscriptionId, tx)
        subscriptionsStorage.addSubscription(key, tx).flatMap {
          case true =>
            workerApi.subscribe(key, tx).flatMap { stream =>
              for {
                _ <- subscriptionsStorage.addStream(key, stream)
                _ <- Concurrent[F].start(
                  stream
                    .evalMap(r => outputQueue.enqueue1(Some(toWebsocketResponse(key.subscriptionId, r))))
                    .compile
                    .drain
                )
              } yield SubscribeResponse(requestId): WebsocketResponse
            }
          case false =>
            (ErrorResponse(requestId, s"Subscription $subscriptionId already exists"): WebsocketResponse).pure[F]
        }
      case UnsubscribeRequest(requestId, subscriptionId, tx) =>
        val key = SubscriptionKey.generate(subscriptionId, tx)
        subscriptionsStorage
          .deleteSubscription(key)
          .flatMap(
            _ => workerApi.unsubscribe(key).map(isOk => UnsubscribeResponse(requestId, isOk))
          )

    }
}

object WorkerWebsocket {

  case class SubscriptionKey private (subscriptionId: String, txHash: String)

  object SubscriptionKey {

    /**
     *
     * @param tx A transaction, to calculate a hash of it.
     *           Hash of a transaction is required because of multiple identical subscriptions.
     */
    def generate(subscriptionId: String, tx: String): SubscriptionKey = {
      new SubscriptionKey(subscriptionId, ByteVector(tx.getBytes()).toHex)
    }
  }

  private[websocket] case class Subscription[F[_]](tx: String, stream: Option[fs2.Stream[F, TendermintResponse]])

  def apply[F[_]: Concurrent: Log](workerApi: WorkerApi[F]): F[WorkerWebsocket[F]] =
    for {
      storage <- SubscriptionStorage()
      queue <- Queue.noneTerminated[F, WebsocketResponse]
    } yield new WorkerWebsocket[F](workerApi, storage, queue)

}
