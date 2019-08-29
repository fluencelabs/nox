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
import cats.effect.{Concurrent, Resource, Timer}
import cats.effect.concurrent.{Deferred, Ref}
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.{Functor, Parallel, Traverse}
import fluence.effects.{Backoff, EffectError}
import fluence.effects.tendermint.rpc.http.{RpcBodyMalformed, RpcError, RpcRequestErrored, TendermintHttpRpc}
import fluence.effects.tendermint.rpc.websocket.TendermintWebsocketRpc
import fluence.log.Log
import fluence.node.MakeResource
import fluence.statemachine.data.{QueryCode, Tx}

import scala.language.higherKinds

class ResponseSubscriberImpl[F[_]: Functor: Timer, G[_]](
  subscribesRef: Ref[F, Map[Tx.Head, ResponsePromise[F]]],
  tendermintRpc: TendermintHttpRpc[F],
  tendermintWRpc: TendermintWebsocketRpc[F],
  appId: Long,
  maxBlocksTries: Int = 3
)(
  implicit F: Concurrent[F],
  P: Parallel[F, G],
  log: Log[F],
  backoff: Backoff[EffectError] = Backoff.default[EffectError]
) extends ResponseSubscriber[F] {

  import io.circe.parser._

  /**
   * Adds a request to query for a response after a block is generated.
   *
   */
  def subscribe(id: Tx.Head): F[Deferred[F, TendermintQueryResponse]] =
    for {
      newResponsePromise <- Deferred[F, TendermintQueryResponse]
      responsePromise <- subscribesRef.modify { subs =>
        subs.get(id).map(rp => (subs, rp.promise)).getOrElse {
          val newPromise = ResponsePromise(id, newResponsePromise)
          (subs + (id -> newPromise), newResponsePromise)
        }
      }
    } yield responsePromise

  /**
   * Subscribes a worker to process subscriptions after each received block.
   *
   */
  override def start(): Resource[F, Unit] =
    log.scope("responseSubscriber") { implicit log =>
      for {
        lastHeight <- Resource.liftF(
          backoff.retry(tendermintRpc.consensusHeight(), e => log.error("retrieving consensus height", e))
        )
        _ <- Log.resource.info("Creating subscription for tendermint blocks")
        blockStream = tendermintWRpc.subscribeNewBlock(lastHeight)
        pollingStream = blockStream
          .evalTap(b => log.debug(s"got block ${b.header.height}"))
          .evalMap(_ => pollResponses(tendermintRpc))
        _ <- MakeResource.concurrentStream(pollingStream)
      } yield ()
    }

  /**
   * Deserializes response and check if they are `ok` or not.
   *
   * @param id session/nonce of request
   */
  private def parseResponse(id: Tx.Head, response: String): EitherT[F, RpcError, TendermintQueryResponse] = {
    for {
      code <- EitherT
        .fromEither(decode[QueryResponseCode](response))
        .leftSemiflatMap(
          err =>
            log
              .error(s"Query response from tendermint is malformed: $response")
              .map(_ => RpcBodyMalformed(err): RpcError)
        )
        .map(_.code.getOrElse(QueryCode.Ok))
    } yield {
      // if code is not 0, 3 or 4 - it is an tendermint error, so we need to return it as is
      // 3, 4 - is a code for pending result
      code match {
        case QueryCode.Pending | QueryCode.NotFound => PendingResponse(id)
        case QueryCode.Ok | _                       => OkResponse(id, response)
      }
    }
  }

  /**
   * Query responses for subscriptions.
   *
   * @param promises list of subscriptions
   * @return all queried responses
   */
  private def queryResponses(
    promises: List[ResponsePromise[F]],
    tendermint: TendermintHttpRpc[F]
  ): F[List[(ResponsePromise[F], TendermintQueryResponse)]] = {
    import cats.syntax.parallel._
    import cats.syntax.list._
    log.scope("responseSubscriber" -> "queryResponses", "app" -> appId.toString) { implicit log =>
      log.debug(s"Polling ${promises.size} promises") >> log.trace(s"promises: ${promises.map(_.id).mkString(" ")}") >>
        promises.map { responsePromise =>
          tendermint
            .query(responsePromise.id.toString, id = "dontcare")
            .flatMap(parseResponse(responsePromise.id, _))
            .map(r => (responsePromise, r))
            .leftMap(err => (responsePromise, err))
        }.map(_.value)
          .toNel
          .map(
            _.parSequence
              .map(
                _.collect {
                  case Right(r) => r
                  case Left((responsePromise, err: RpcRequestErrored)) =>
                    (responsePromise, OkResponse(responsePromise.id, err.error))
                  case Left((responsePromise, rpcError)) =>
                    (responsePromise, RpcErrorResponse(responsePromise.id, rpcError): TendermintQueryResponse)
                }
              )
          )
          .getOrElse(List.empty.pure[F])
    }
  }

  /**
   * Checks all responses, completes all `ok` responses, increments `tries` counter for `bad` responses,
   * updates state of promises.
   *
   */
  private def updateSubscribesByResult(responses: List[(ResponsePromise[F], TendermintQueryResponse)]): F[Unit] = {
    import cats.instances.list._
    for {
      completionList <- subscribesRef.modify { subsMap =>
        val (taskList, updatedSubs) = responses.foldLeft((List.empty[F[Unit]], subsMap)) {
          case ((taskList, subs), response) =>
            response match {
              case (promise, r @ OkResponse(_, _)) =>
                (promise.complete(r) :: taskList, subs - promise.id)
              case (promise, r @ (RpcErrorResponse(_, _) | PendingResponse(_))) =>
                if (promise.tries + 1 >= maxBlocksTries) {
                  // return TimedOutResponse after `tries` PendingResponses
                  val response = r match {
                    case PendingResponse(id) => TimedOutResponse(id, promise.tries + 1)
                    case resp                => resp
                  }
                  (promise.complete(response) :: taskList, subs - r.id)
                } else (taskList, subs + (r.id -> promise.copy(tries = promise.tries + 1)))
              case (promise, r @ TimedOutResponse(_, _)) =>
                (
                  log.error("Unexpected. TimedOutResponse couldn't be here.") *> promise.complete(r) :: taskList,
                  subs - promise.id
                )
            }
        }
        (updatedSubs, taskList)
      }
      _ <- Traverse[List].traverse(completionList)(identity)
    } yield ()
  }

  /**
   * Get all subscriptions for an app by `appId`, queries responses from tendermint.
   */
  private def pollResponses(tendermintRpc: TendermintHttpRpc[F]): F[Unit] =
    for {
      responsePromises <- subscribesRef.get
      _ <- queryResponses(responsePromises.values.toList, tendermintRpc).flatMap(updateSubscribesByResult)
    } yield ()
}

object ResponseSubscriberImpl {

  def apply[F[_]: Log: Concurrent: Timer, G[_]](
    tendermintRpc: TendermintHttpRpc[F],
    tendermintWRpc: TendermintWebsocketRpc[F],
    appId: Long,
    maxBlocksTries: Int = ResponseSubscriber.MaxBlockTries
  )(
    implicit P: Parallel[F, G]
  ): F[ResponseSubscriberImpl[F, G]] =
    Ref
      .of[F, Map[Tx.Head, ResponsePromise[F]]](
        Map.empty[Tx.Head, ResponsePromise[F]]
      )
      .map(r => new ResponseSubscriberImpl(r, tendermintRpc, tendermintWRpc, appId, maxBlocksTries))

}
