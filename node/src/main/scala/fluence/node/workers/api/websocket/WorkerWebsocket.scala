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

import cats.Traverse
import cats.data.EitherT
import cats.effect.Concurrent
import cats.effect.concurrent.Ref
import fluence.log.Log
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import fluence.node.workers.api.WorkerApi
import fluence.node.workers.api.websocket.WorkerWebsocket.SubscriptionKey
import fluence.node.workers.subscription.StoredProcedureExecutor.TendermintResponse
import fluence.node.workers.subscription.{
  OkResponse,
  PendingResponse,
  RpcErrorResponse,
  TendermintQueryResponse,
  TimedOutResponse,
  TxAwaitError
}
import fs2.concurrent.{NoneTerminatedQueue, Queue}
import io.circe.parser.parse
import io.circe.syntax._

import scala.language.higherKinds

/**
 * The layer between messages from Websocket and WorkerAPI.
 *
 * @param subscriptions on transactions after each block
 * @param outputQueue for sending messages into websocket
 */
class WorkerWebsocket[F[_]: Concurrent: Log](
  workerApi: WorkerApi[F],
  subscriptions: Ref[F, Map[SubscriptionKey, Option[fs2.Stream[F, TendermintResponse]]]],
  outputQueue: NoneTerminatedQueue[F, Either[TxAwaitError, TendermintQueryResponse]]
) {
  import WebsocketRequests._
  import WebsocketResponses._

  /**
   * Unsubscribes from all subscriptions and closes the queue.
   *
   */
  def closeWebsocket(): F[Unit] = {
    import cats.instances.list._
    for {
      subs <- subscriptions.get
import cats.syntax.traversable._
import cats.instances.list._
      _ <- subs.keys.toList.traverse(workerApi.unsubscribe)
        workerApi.unsubscribe(key)
      }
      _ <- Traverse[List].traverse(tasks.toList)(identity)
      _ <- outputQueue.enqueue1(None)
    } yield {}
  }

  private def addStream(key: SubscriptionKey, stream: fs2.Stream[F, TendermintResponse]): F[Boolean] =
    subscriptions.modify { subs =>
      subs.get(key) match {
        case Some(v) => (subs.updated(key, v), true)
        case None    => (subs, false)
      }
    }

  private def checkAndAddSubscription(key: SubscriptionKey): F[Boolean] =
    for {
      success <- subscriptions.modify { subs =>
        subs.get(key) match {
          case Some(_) => (subs, false)
          case None =>
            (subs + (key -> None), true)
        }
      }
    } yield success

  private def deleteSubscription(key: SubscriptionKey): F[Unit] = subscriptions.update(_ - key)

  /**
   * Parse input and call WebsocketAPI.
   * @param input messages from websocket
   */
  def processRequest(input: String): F[String] = {
    val result = for {
      request <- EitherT
        .fromEither(parse(input).flatMap(_.as[WebsocketRequest]))
      response <- EitherT.liftF[F, io.circe.Error, WebsocketResponse](callApi(request))
    } yield response
    result.value.map {
      case Right(v)    => v
      case Left(error) => ErrorResponse("", s"Cannot parse msg. Error: $error, msg: $input")
    }.map(_.asJson.noSpaces)
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
          .map {
            case Right(OkResponse(_, response))    => TxWaitResponse(requestId, response)
            case Right(RpcErrorResponse(_, error)) => ErrorResponse(requestId, error.getMessage)
            case Right(TimedOutResponse(_, tries)) =>
              ErrorResponse(requestId, s"Cannot get response after $tries generated blocks")
            case Right(PendingResponse(_)) => ErrorResponse(requestId, s"Unexpected error.")
            case Left(error)               => ErrorResponse(requestId, error.msg)
          }
      case LastManifestRequest(requestId) =>
        workerApi.lastManifest().map(block => LastManifestResponse(requestId, block.map(_.jsonString)))
      case StatusRequest(requestId) =>
        workerApi.tendermintStatus().map {
          case Right(status) => StatusResponse(requestId, status)
          case Left(error)   => ErrorResponse(requestId, error.getMessage)
        }
      case P2pPortRequest(requestId) => workerApi.p2pPort().map(port => P2pPortResponse(requestId, port))
      case SubscribeRequest(requestId, subscriptionId, tx) =>
        val key = SubscriptionKey(subscriptionId, tx)
        checkAndAddSubscription(key).flatMap {
          case true =>
            workerApi.subscribe(key).flatMap { stream =>
              for {
                _ <- addStream(key, stream)
                _ = {
                  stream
                    .evalMap(r => outputQueue.enqueue1(Option(r)))
                    .compile
                    .drain
                }
              } yield SubscribeResponse(requestId): WebsocketResponse
            }
          case false =>
            (ErrorResponse(requestId, s"Subscription $subscriptionId already exists"): WebsocketResponse).pure[F]
        }
      case UnsubscribeRequest(requestId, subscriptionId, tx) =>
        val key = SubscriptionKey(subscriptionId, tx)
        deleteSubscription(key).flatMap(
          _ => workerApi.unsubscribe(key).map(isOk => UnsubscribeResponse(requestId, isOk))
        )

    }
}

object WorkerWebsocket {

  case class SubscriptionKey(subscriptionId: String, tx: String)

  def apply[F[_]: Concurrent: Log](workerApi: WorkerApi[F]): F[WorkerWebsocket[F]] =
    for {
      subs <- Ref.of(Map.empty[SubscriptionKey, Option[fs2.Stream[F, TendermintResponse]]])
      queue <- Queue.noneTerminated[F, Either[TxAwaitError, TendermintQueryResponse]]
    } yield new WorkerWebsocket[F](workerApi, subs, queue)

}
