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

import cats.Monad
import cats.data.EitherT
import cats.effect.Concurrent
import cats.effect.concurrent.Ref
import fluence.log.Log
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.applicative._
import fluence.node.workers.api.WorkerApi
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
 */
class WorkerWebsocket[F[_]: Concurrent: Log](
  workerApi: WorkerApi[F],
  subscriptions: Ref[F, Map[String, Option[fs2.Stream[F, Option[TendermintResponse]]]]],
  outputQueue: NoneTerminatedQueue[F, Either[TxAwaitError, TendermintQueryResponse]]
) {
  import WebsocketRequests._
  import WebsocketResponses._

  private def addStream(subscriptionId: String, stream: fs2.Stream[F, Option[TendermintResponse]]): F[Boolean] = {
    subscriptions.modify { subs =>
      subs.get(subscriptionId) match {
        case Some(v) => (subs.updated(subscriptionId, v), true)
        case None    => (subs, false)
      }
    }
  }

  private def checkAndAddSubscription(subscriptionId: String): F[Boolean] = {
    for {
      success <- subscriptions.modify { subs =>
        subs.get(subscriptionId) match {
          case Some(_) => (subs, false)
          case None =>
            (subs + (subscriptionId -> None), true)
        }
      }
    } yield success
  }

  private def deleteSubscription(subscriptionId: String): F[Unit] = subscriptions.update(_ - subscriptionId)

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

  private def callApi(input: WebsocketRequest): F[WebsocketResponse] = {
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
        checkAndAddSubscription(subscriptionId).flatMap {
          case true =>
            workerApi.subscribe(subscriptionId, tx).flatMap { stream =>
              for {
                _ <- addStream(subscriptionId, stream)
                _ = {
                  stream.map(r => outputQueue.enqueue1(r)).compile.drain
                }
              } yield SubscribeResponse(requestId): WebsocketResponse
            }
          case false =>
            (ErrorResponse(requestId, s"Subscription $subscriptionId already exists"): WebsocketResponse).pure[F]
        }
      case UnsubscribeRequest(requestId, subscriptionId, tx) =>
        workerApi.unsubscribe(subscriptionId, tx).map(isOk => UnsubscribeResponse(requestId, isOk))
    }
  }
}

object WorkerWebsocket {

  def apply[F[_]: Concurrent: Log](workerApi: WorkerApi[F]): F[WorkerWebsocket[F]] =
    for {
      subs <- Ref.of(Map.empty[String, Option[fs2.Stream[F, Option[TendermintResponse]]]])
      queue <- Queue.noneTerminated[F, Either[TxAwaitError, TendermintQueryResponse]]
    } yield new WorkerWebsocket[F](workerApi, subs, queue)

}
