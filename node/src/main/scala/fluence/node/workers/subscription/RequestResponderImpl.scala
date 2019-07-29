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
import cats.syntax.apply._
import cats.{Functor, Parallel, Traverse}
import fluence.effects.{Backoff, EffectError}
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.tendermint.rpc.http.{RpcBodyMalformed, RpcError, TendermintHttpRpc}
import fluence.log.Log
import fluence.node.MakeResource
import fluence.statemachine.data.Tx

import scala.language.higherKinds

class RequestResponderImpl[F[_]: Functor: Timer, G[_]](
  subscribesRef: Ref[F, List[ResponsePromise[F]]],
  tendermint: TendermintRpc[F],
  appId: Long,
  maxBlocksTries: Int = 3
)(
  implicit F: Concurrent[F],
  P: Parallel[F, G],
  log: Log[F],
  backoff: Backoff[EffectError] = Backoff.default[EffectError]
) extends RequestResponder[F] {

  import io.circe.parser._

  /**
   * Adds a request to query for a response after a block is generated.
   *
   */
  def subscribe(id: Tx.Head): F[Deferred[F, TendermintQueryResponse]] =
    for {
      responsePromise <- Deferred[F, TendermintQueryResponse]
      _ <- subscribesRef.update { list =>
        val newPromise = ResponsePromise(id, responsePromise)
        list :+ newPromise
      }
    } yield responsePromise

  /**
   * Subscribes a worker to process subscriptions after each received block.
   *
   */
  override def subscribeForWaitingRequests(): Resource[F, Unit] =
    log.scope(("requestResponder", "subscribeForWaitingRequests")) { implicit log =>
      for {
        lastHeight <- Resource.liftF(
          backoff.retry(tendermint.consensusHeight(), e => log.error("retrieving consensus height", e))
        )
        _ <- Resource.liftF(log.info("Creating subscription for tendermint blocks"))
        blockStream = tendermint.subscribeNewBlock(lastHeight)
        pollingStream = blockStream.evalMap { _ =>
          pollResponses(tendermint)
        }
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
        .leftMap(err => RpcBodyMalformed(err): RpcError)
        .map(_.code)
    } yield {
      // if code is not 0, 3 or 4 - it is an tendermint error, so we need to return it as is
      // 3, 4 - is a code for pending result
      if (code == 0 || (code != 3 && code != 4)) {
        OkResponse(id, response)
      } else {
        PendingResponse(id, response)
      }
    }
  }

  /**
   * Query responses for subscriptions.
   *
   * @param promises list of subscriptions
   * @return all queried responses
   */
  private def queryResponses(promises: List[ResponsePromise[F]],
                             tendermint: TendermintHttpRpc[F]): F[List[TendermintQueryResponse]] = {
    import cats.syntax.parallel._
    import cats.syntax.list._
    log.scope("requestResponder" -> "queryResponses", "app" -> appId.toString) { implicit log =>
      log.trace(s"Polling ${promises.size} promises") *>
        promises.map { responsePromise =>
          tendermint
            .query(responsePromise.id.toString, id = "dontcare")
            .flatMap(parseResponse(responsePromise.id, _))
            .leftMap(err => (responsePromise.id, err))
        }.map(_.value)
          .toNel
          .map(
            _.parSequence
              .map(
                _.collect {
                  case Right(r) => r
                  case Left(err) =>
                    RpcErrorResponse(err._1, err._2): TendermintQueryResponse
                }
              )
          )
          .getOrElse(F.pure(List.empty))
    }
  }

  /**
   * Checks if response is ok. If response code is `pending` or there is an error, increment the `tries` counter.
   * If number of tries more than `maxBlocksTries`, complete promise with last response or an error.
   *
   * @param subscriptions existent subscriptions
   * @param id session/nonce of a submission
   * @param completionList accumulator of tasks to complete subscription
   * @return
   */
  private def checkResponseCompletion(
    subscriptions: Map[Tx.Head, ResponsePromise[F]],
    id: Tx.Head,
    response: TendermintQueryResponse,
    completionList: List[F[Unit]]
  ): (List[F[Unit]], Map[Tx.Head, ResponsePromise[F]]) = {
    subscriptions
      .get(id)
      .map { rp =>
        if (rp.tries + 1 >= maxBlocksTries) (completionList :+ rp.promise.complete(response), subscriptions - id)
        else (completionList, subscriptions + (id -> rp.copy(tries = rp.tries + 1)))
      }
      .getOrElse((completionList, subscriptions))
  }

  /**
   * Checks all responses, completes all `ok` responses, increments `tries` counter for `bad` responses,
   * updates state of promises.
   *
   */
  private def updateSubscribesByResult(responses: List[TendermintQueryResponse]): F[Unit] = {
    import cats.instances.list._
    for {
      completionList <- subscribesRef.modify { m =>
        val subsMap = m.map(v => v.id -> v).toMap
        val emptyTaskList = List.empty[F[Unit]]
        val updatedMap = responses.foldLeft((emptyTaskList, subsMap)) {
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
        (updatedMap._2.values.toList, updatedMap._1)
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
      _ <- queryResponses(responsePromises, tendermintRpc).flatMap(updateSubscribesByResult)
    } yield ()
}

object RequestResponderImpl {

  def apply[F[_]: Log: Concurrent: Timer, G[_]](
    tendermint: TendermintRpc[F],
    appId: Long,
    maxBlocksTries: Int = 3
  )(
    implicit P: Parallel[F, G]
  ): F[RequestResponderImpl[F, G]] =
    Ref
      .of[F, List[ResponsePromise[F]]](
        List.empty[ResponsePromise[F]]
      )
      .map(r => new RequestResponderImpl(r, tendermint, appId, maxBlocksTries))

}
