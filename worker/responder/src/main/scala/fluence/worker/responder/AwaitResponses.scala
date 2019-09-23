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

package fluence.worker.responder

import cats.effect.concurrent.{Deferred, Ref}
import cats.effect.{Concurrent, Resource, Timer}
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Parallel, Traverse}
import fluence.bp.tx.{Tx, TxsBlock}
import fluence.effects.resources.MakeResource
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.query.QueryCode
import fluence.worker.api.Worker
import fluence.worker.responder.resp._
import scodec.bits.ByteVector

import scala.language.higherKinds

class AwaitResponses[F[_]: Concurrent: Parallel: Timer, B: TxsBlock](
  worker: Worker.AuxP[F, B],
  subscribesRef: Ref[F, Map[Tx.Head, ResponsePromise[F]]],
  maxBlocksTries: Int
)(implicit backoff: Backoff[EffectError]) {

  import worker._

  /**
   * Adds a request to query for a response after a block is generated.
   *
   */
  def await(id: Tx.Head)(implicit log: Log[F]): F[Deferred[F, AwaitedResponse]] =
    for {
      newResponsePromise <- Deferred[F, AwaitedResponse]
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
  private def start()(implicit log: Log[F]): Resource[F, Unit] =
    log.scope("responseSubscriber") { implicit log =>
      for {
        _ <- Log.resource.info("Creating subscription for tendermint blocks")
        blockStream = producer.blockStream(fromHeight = None)
        pollingStream = blockStream
          .evalTap(b => log.debug(s"got block ${TxsBlock[B].height(b)}"))
          .filter(nonEmptyBlock)
          .evalMap(_ => pollResponses(machine))
        _ <- MakeResource.concurrentStream(pollingStream)
      } yield ()
    }

  /**
   * @return true if block contains any non-pubsub txs, false otherwise
   */
  private def nonEmptyBlock(block: B): Boolean = TxsBlock[B].txs(block).exists(!isRepeatTx(_))

  private def isRepeatTx(tx: ByteVector) = tx.startsWith(AwaitResponses.AwaitSessionPrefixBytes)

  /**
   * Send query to state machine and convert result to AwaitedResponse
   */
  private def query(txHead: Tx.Head)(implicit log: Log[F]): F[AwaitedResponse] =
    machine
      .query(txHead.toString)
      .value
      .map {
        case Left(e)                                  => RpcErrorResponse(txHead, e)
        case Right(r) if r.code == QueryCode.Pending  => PendingResponse(txHead)
        case Right(r) if r.code == QueryCode.NotFound => PendingResponse(txHead)
        case Right(r)                                 => OkResponse(txHead, r.toResponseString())
        // TODO: is it ok to return CannotParseHeader & Dropped as OkResponse?
      }

  private def logPromises(promises: List[ResponsePromise[F]])(implicit log: Log[F]): F[Unit] =
    log.debug(s"Polling ${promises.size} promises") >> log.trace(s"promises: ${promises.map(_.id).mkString(" ")}")

  /**
   * Query responses for subscriptions.
   *
   * @param promises list of subscriptions
   * @return all queried responses
   */
  private def queryResponses(
    promises: List[ResponsePromise[F]],
    machine: StateMachine[F]
  )(implicit log: Log[F]): F[List[(ResponsePromise[F], AwaitedResponse)]] = {
    import cats.syntax.list._
    import cats.syntax.parallel._
    log.scope("queryResponses" -> "") { implicit log =>
      logPromises(promises) >>
        promises
          .map(p => query(p.id).tupleLeft(p))
          .toNel
          .map(_.parSequence.map(_.toList))
          .getOrElse(List.empty.pure[F])
    }
  }

  /**
   * Checks all responses, completes all `ok` responses, increments `tries` counter for `bad` responses,
   * updates state of promises.
   *
   */
  private def updateSubscribesByResult(
    responses: List[(ResponsePromise[F], AwaitedResponse)]
  )(implicit log: Log[F]): F[Unit] = {
    import cats.instances.list._
    for {
      completionList <- subscribesRef.modify { subsMap =>
        val (taskList, updatedSubs) = responses.foldLeft((List.empty[F[Unit]], subsMap)) {
          case ((taskList, subs), response) =>
            response match {
              case (promise, r: OkResponse) =>
                (promise.complete(r) :: taskList, subs - promise.id)

              case (promise, r @ (_: RpcErrorResponse | _: PendingResponse)) =>
                if (promise.tries + 1 >= maxBlocksTries) {
                  // return TimedOutResponse after `tries` PendingResponses
                  val response = r match {
                    case PendingResponse(id) => TimedOutResponse(id, promise.tries + 1)
                    case resp                => resp
                  }
                  (promise.complete(response) :: taskList, subs - r.id)
                } else (taskList, subs + (r.id -> promise.copy(tries = promise.tries + 1)))

              case (promise, r: TimedOutResponse) =>
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
  private def pollResponses(machine: StateMachine[F])(implicit log: Log[F]): F[Unit] =
    for {
      responsePromises <- subscribesRef.get
      responses <- queryResponses(responsePromises.values.toList, machine)
      _ <- updateSubscribesByResult(responses)
    } yield ()

}

object AwaitResponses {

  val MaxBlocksTries = 10

  val RepeatSessionPrefix = "repeat"

  private val AwaitSessionPrefixBytes = ByteVector(RepeatSessionPrefix.getBytes)

  def make[F[_]: Parallel: Concurrent: Log: Timer, B: TxsBlock](
    worker: Worker.AuxP[F, B],
    maxTries: Int = MaxBlocksTries
  )(
    implicit backoff: Backoff[EffectError]
  ): Resource[F, AwaitResponses[F, B]] =
    MakeResource
      .refOf(Map.empty[Tx.Head, ResponsePromise[F]])
      .map(new AwaitResponses[F, B](worker, _, maxTries))
      .flatTap(_.start())
}
