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

import cats.data.{EitherT, NonEmptyList}
import cats.effect.Concurrent
import cats.effect.concurrent.Ref
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.list._
import cats.{Functor, Parallel, Traverse}
import fluence.effects.tendermint.rpc.{QueryResponseCode, TendermintRpc}
import fluence.effects.tendermint.rpc.http.{RpcBodyMalformed, RpcError, TendermintHttpRpc}
import fluence.log.LogFactory
import fluence.statemachine.data.Tx

import scala.language.higherKinds

class RequestResponderImpl[F[_]: LogFactory: Functor, G[_]](
  subscribesRef: Ref[F, Map[Long, NonEmptyList[ResponsePromise[F]]]],
  maxBlocksTries: Int = 3
)(
  implicit F: Concurrent[F],
  P: Parallel[F, G]
) extends RequestResponder[F] {

  import io.circe.parser._

  def parseResponse(id: Tx.Head, response: String): EitherT[F, RpcError, TendermintQueryResponse] = {
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

  def queryResponses(appId: Long,
                     promises: NonEmptyList[ResponsePromise[F]],
                     tendermint: TendermintHttpRpc[F]): F[List[TendermintQueryResponse]] = {
    import cats.syntax.parallel._
    LogFactory[F].init("requestResponder" -> "queryResponses", "app" -> appId.toString) >>= { implicit log =>
      promises.map { responsePromise =>
        tendermint
          .query(responsePromise.id.toString, "", id = "dontcare")
          .flatMap(parseResponse(responsePromise.id, _))
          .leftMap(err => (responsePromise.id, err))
      }.map(_.value)
        .parSequence
        .map(_.collect {
          case Right(r)  => r
          case Left(err) => RpcErrorResponse(err._1, err._2): TendermintQueryResponse
        })
    }
  }

  def checkResponseCompletion(subs: Map[Tx.Head, ResponsePromise[F]],
                              id: Tx.Head,
                              response: TendermintQueryResponse,
                              taskList: List[F[Unit]]): (List[F[Unit]], Map[Tx.Head, ResponsePromise[F]]) = {
    subs
      .get(id)
      .map { rp =>
        if (rp.tries + 1 >= maxBlocksTries) (taskList :+ rp.promise.complete(response), subs - id)
        else (taskList, subs + (id -> rp.copy(tries = rp.tries + 1)))
      }
      .getOrElse((taskList, subs))
  }

  def updateSubscribesByResult(appId: Long, result: List[TendermintQueryResponse]): F[Unit] = {
    import cats.instances.list._
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
  }

  def pollResponses(appId: Long, tendermintRpc: TendermintHttpRpc[F]): F[Unit] =
    for {
      subscribed <- getSubscribed(appId)
      _ <- subscribed match {
        case Some(responsePromises) =>
          queryResponses(appId, responsePromises, tendermintRpc).flatMap(updateSubscribesByResult(appId, _))
        case None => F.unit
      }
    } yield ()

  def getSubscribed(appId: Long): F[Option[NonEmptyList[ResponsePromise[F]]]] =
    subscribesRef.get.map(_.get(appId))
}

object RequestResponderImpl {

  def apply[F[_]: LogFactory: Concurrent, G[_]](subscribesRef: Ref[F, Map[Long, NonEmptyList[ResponsePromise[F]]]],
                                                maxBlocksTries: Int = 3)(
    implicit P: Parallel[F, G]
  ): RequestResponderImpl[F, G] =
    new RequestResponderImpl(subscribesRef, maxBlocksTries)
}
