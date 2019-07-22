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
import cats.{Functor, Parallel}
import cats.effect.Concurrent
import cats.effect.concurrent.{Deferred, Ref}
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.effects.tendermint.rpc.http.{RpcBodyMalformed, RpcError}
import fluence.effects.tendermint.rpc.{QueryResponseCode, TendermintRpc}
import fluence.log.Log
import fluence.node.workers.{WorkersHttp, WorkersPool}

import scala.language.higherKinds

case class RequestId(session: String, nonce: Long) {
  override def toString: String = s"$session/$nonce"
}
case class ResponsePromise[F](id: RequestId, promise: Deferred[F, Option[String]], tries: Int = 0)

trait TendermintResponse
case class OkResponse(id: RequestId, r: Option[String]) extends TendermintResponse
case class RpcErrorResponse(id: RequestId, r: RpcError) extends TendermintResponse
case class PendingResponse(id: RequestId) extends TendermintResponse

class RequestSubscriber[F[_]: Concurrent](
  subscribesRef: Ref[F, Map[Long, Map[RequestId, ResponsePromise[F]]]]
) {

  def subscribe(appId: Long, id: RequestId): F[Deferred[F, Option[String]]] =
    for {
      responsePromise <- Deferred[F, Option[String]]
      _ <- subscribesRef.update { m =>
        val newPromise = ResponsePromise(id, responsePromise)
        m.updated(appId, m.get(appId).map(_ + (newPromise.id -> newPromise)).getOrElse(Map((newPromise, Nil))))
      }
    } yield responsePromise

  def getSubscribedAndClear(appId: Long): F[Option[Map[RequestId, ResponsePromise[F]]]] =
    subscribesRef.modify(m => ((m - appId), m.get(appId)))

  def getSubscribed(appId: Long): F[Option[Map[RequestId, ResponsePromise[F]]]] =
    subscribesRef.get.map(_.get(appId))
}

object RequestSubscriber {

  def apply[F[_]: Concurrent](): F[RequestSubscriber[F]] =
    for {
      subscribes <- Ref.of(Map.empty[Long, Map[RequestId, ResponsePromise[F]]])
    } yield new RequestSubscriber(subscribes)
}

class Responder[F[_]: Log: Functor, G[_]](subscribesRef: Ref[F, Map[Long, NonEmptyList[ResponsePromise[F]]]],
                                          tendermint: TendermintRpc[F],
                                          pool: WorkersPool[F],
                                          maxBlocksTries: Int = 3)(
  implicit F: Concurrent[F],
  P: Parallel[F, G]
) {

  import io.circe.parser._

  def parseResponse(id: RequestId, response: String): EitherT[F, RpcError, TendermintResponse] = {
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
        PendingResponse(id)
      }
    }
  }

  def queryResponses(appId: Long, promises: NonEmptyList[ResponsePromise[F]]): F[List[TendermintResponse]] = {
    import cats.syntax.parallel._

    import cats.syntax.list._
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

  def updateSubscribesByResult(appId: Long, result: List[TendermintResponse]) =
    for {
      _ <- subscribesRef.get
      _ <- subscribesRef.update { m =>
        m.get(appId).map { promises =>
          }
        result.foreach {
          case OkResponse(id, r)       => m
          case RpcErrorResponse(id, r) =>
          case PendingResponse(id)     =>
        }
      }
    } yield ()

  def poll(appId: Long): F[Unit] =
    for {
      subscribed <- getSubscribed(appId)
      _ <- subscribed match {
        case Some(responsePromises) =>
          queryResponses(appId, responsePromises).map
        case None => F.unit
      }
    } yield ()

  def subscribe(appId: Long, id: RequestId): F[Deferred[F, String]] =
    for {
      responsePromise <- Deferred[F, String]
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
