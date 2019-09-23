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

import cats.Parallel
import cats.effect.concurrent.{Deferred, Ref}
import cats.effect.{Concurrent, Resource, Timer}
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
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
   * Get all subscriptions for an app by `appId`, queries responses from tendermint.
   */
  private def pollResponses(machine: StateMachine[F])(implicit log: Log[F]): F[Unit] = {
    import cats.instances.list._
    import cats.syntax.traverse._

    for {
      responsePromises <- subscribesRef.get
      (complete, retry) <- queryResponses(responsePromises.values.toList, machine)
      _ <- updateSubscriptions(complete.map(_._1.id), retry)
      // complete promises with responses
      _ <- complete.traverse { case (p, r) => p.complete(r) }
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
   * Query responses for all existing subscriptions
   *
   * @param promises list of subscriptions
   * @param machine Statemachine api
   * @return All responses partitioned into two lists: completed and to be retried
   */
  private def queryResponses(
    promises: List[ResponsePromise[F]],
    machine: StateMachine[F]
  )(implicit log: Log[F]) = {
    import cats.syntax.list._
    import cats.syntax.parallel._
    log.scope("queryResponses" -> "") { implicit log =>
      logPromises(promises) >>
        promises
          .map(p => query(p.id).tupleLeft(p))
          .toNel
          .map(_.parSequence.map(_.toList))
          .getOrElse(List.empty[(ResponsePromise[F], AwaitedResponse)].pure[F])
          .map(partition)
    }
  }

  /**
   * Partitions promises into two lists: to complete and to retry.
   *
   * @param responses List of responses to partition
   * @return tuple of lists: left component is a list of promises to be completed; right – to be retried.
   */
  private def partition(responses: List[(ResponsePromise[F], AwaitedResponse)])(implicit log: Log[F]) = {
    import cats.syntax.either._

    def tout(p: ResponsePromise[F]) = TimedOutResponse(p.id, p.tries)
    def inc(p: ResponsePromise[F]) = p.copy(tries = p.tries + 1)

    // TODO: use partitionMap from 2.13
    val (left, right) = responses.map {
      case t @ (_, _: OkResponse | _: TimedOutResponse)          => t.asLeft // Got response
      case (p, e: RpcErrorResponse) if p.tries == maxBlocksTries => (p, e).asLeft // Error, no more tries
      case (p, _: PendingResponse) if p.tries == maxBlocksTries  => (p, tout(p)).asLeft // No response, no more tries
      case (p, _: RpcErrorResponse | _: PendingResponse)         => (p.id, inc(p)).asRight // Error or no response, retry
    }.partition(_.isLeft)

    (left.map(_.left.get), right.map(_.right.get))
  }

  /**
   * Removes completed and adds retries
   */
  private def updateSubscriptions(completed: List[Tx.Head], retries: List[(Tx.Head, ResponsePromise[F])]) =
    subscribesRef.update(_ -- completed ++ retries)
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
