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

package fluence.node

import cats.data.{EitherT, NonEmptyList}
import cats.{Functor, Parallel, Traverse}
import cats.effect.Concurrent
import cats.effect.concurrent.{Deferred, Ref}
import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.syntax.list._
import fluence.effects.tendermint.rpc.http.{RpcBodyMalformed, RpcError}
import fluence.effects.tendermint.rpc.{QueryResponseCode, TendermintRpc}
import fluence.log.{Log, LogFactory}
import fluence.node.workers.{WorkersHttp, WorkersPool}
import fluence.statemachine.data.Tx

import scala.language.higherKinds

case class ResponsePromise[F](id: Tx.Head, promise: Deferred[F, TendermintResponse], tries: Int = 0)

trait TendermintResponse
case class OkResponse(id: Tx.Head, r: Option[String]) extends TendermintResponse
case class RpcErrorResponse(id: Tx.Head, r: RpcError) extends TendermintResponse
case class PendingResponse(id: Tx.Head, r: String) extends TendermintResponse

class RequestResponder[F[_]: LogFactory: Functor, G[_]](
  subscribesRef: Ref[F, Map[Long, NonEmptyList[ResponsePromise[F]]]],
  pool: WorkersPool[F],
  maxBlocksTries: Int = 3
)(
  implicit F: Concurrent[F],
  P: Parallel[F, G]
) {

  import io.circe.parser._

  def parseResponse(id: Tx.Head, response: String): EitherT[F, RpcError, TendermintResponse] = {
    for {
      code <- EitherT
        .fromEither(decode[QueryResponseCode](response))
        .leftMap(err => RpcBodyMalformed(err): RpcError)
        .map(_.code)
    } yield {
      // if code is not 0, 3 or 4 - it is an tendermint error, so we need to return it as is
      // 3, 4 - is a code for pending result
      if (code == 0 || (code != 3 && code != 4)) {
        OkResponse(id, Option(response))
      } else {
        PendingResponse(id, response)
      }
    }
  }

  def queryResponses(appId: Long, promises: NonEmptyList[ResponsePromise[F]]): F[List[TendermintResponse]] = {
    import cats.syntax.parallel._
    LogFactory[F].init("requestResponder" -> "queryResponses", "app" -> appId.toString) >>= { implicit log =>
      promises.map { responsePromise =>
        (for {
          responseOpt <- WorkersHttp.withTendermintRaw(pool, appId)(
            _.query(responsePromise.id.toString, "", id = "dontcare")
          )
          response <- responseOpt match {
            case Some(res) => parseResponse(responsePromise.id, responseOpt.get)
            case None      => EitherT.pure[F, RpcError](OkResponse(responsePromise.id, None): TendermintResponse)
          }
        } yield response).leftMap(err => (responsePromise.id, err))
      }.map(_.value)
        .parSequence
        .map(_.collect {
          case Right(r)  => r
          case Left(err) => RpcErrorResponse(err._1, err._2): TendermintResponse
        })
    }
  }

  def checkResponseCompletion(subs: Map[Tx.Head, ResponsePromise[F]],
                              id: Tx.Head,
                              response: TendermintResponse,
                              taskList: List[F[Unit]]): (List[F[Unit]], Map[Tx.Head, ResponsePromise[F]]) = {
    subs
      .get(id)
      .map { rp =>
        if (rp.tries + 1 >= maxBlocksTries) (taskList :+ rp.promise.complete(response), subs - id)
        else (taskList, subs + (id -> rp.copy(tries = rp.tries + 1)))
      }
      .getOrElse((taskList, subs))
  }

  import cats.instances.list._

  def updateSubscribesByResult(appId: Long, result: List[TendermintResponse]): F[Unit] =
    for {
      completionList <- subscribesRef.modify { m =>
        val subMap = m(appId).toList.map(v => v.id -> v).toMap
        val emptyTaskList = List.empty[F[Unit]]
        val updatedMap = result.foldLeft((emptyTaskList, subMap)) {
          case ((taskList, subs), response) =>
            response match {
              case r @ OkResponse(id, _) =>
                (subs
                   .get(id)
                   .map { rp =>
                     taskList :+ rp.promise.complete(r)
                   }
                   .getOrElse(taskList),
                 subs - id)
              case r @ RpcErrorResponse(id, _) =>
                checkResponseCompletion(subs, id, r, taskList)
              case r @ PendingResponse(id, _) =>
                checkResponseCompletion(subs, id, r, taskList)
            }
        }
        (updatedMap._2.values.toList.toNel.map(um => m + (appId -> um)).getOrElse(m - appId), updatedMap._1)
      }
      _ <- Traverse[List].traverse(completionList)(identity)
    } yield ()

  def poll(appId: Long): F[Unit] =
    for {
      subscribed <- getSubscribed(appId)
      _ <- subscribed match {
        case Some(responsePromises) =>
          queryResponses(appId, responsePromises).flatMap(updateSubscribesByResult(appId, _))
        case None => F.unit
      }
    } yield ()

  def subscribe(appId: Long, id: Tx.Head): F[Deferred[F, TendermintResponse]] =
    for {
      responsePromise <- Deferred[F, TendermintResponse]
      _ <- subscribesRef.update { m =>
        val newPromise = ResponsePromise(id, responsePromise)
        m.updated(appId, m.get(appId).map(_ :+ newPromise).getOrElse(NonEmptyList(newPromise, Nil)))
      }
    } yield responsePromise

  def getSubscribedAndClear(appId: Long): F[Option[NonEmptyList[ResponsePromise[F]]]] =
    subscribesRef.modify(m => ((m - appId), m.get(appId)))

  def getSubscribed(appId: Long): F[Option[NonEmptyList[ResponsePromise[F]]]] =
    subscribesRef.get.map(_.get(appId))
}

object RequestResponder {

  def apply[F[_]: LogFactory: Concurrent, G[_]](pool: WorkersPool[F], maxBlocksTries: Int = 3)(
    implicit P: Parallel[F, G]
  ): F[RequestResponder[F, G]] =
    for {
      subscribes <- Ref.of(Map.empty[Long, NonEmptyList[ResponsePromise[F]]])
    } yield new RequestResponder(subscribes, pool, maxBlocksTries)
}
