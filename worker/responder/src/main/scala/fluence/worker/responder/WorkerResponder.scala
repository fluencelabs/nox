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
import cats.effect.{Concurrent, Resource, Timer}
import fluence.bp.api.{BlockProducer, BlockStream}
import fluence.bp.tx.TxsBlock
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fluence.statemachine.api.StateMachine
import fluence.worker.responder.repeat.RepeatOnEveryBlock
import shapeless._

import scala.language.higherKinds

/**
 * Main patterns to get response for a given transaction
 *
 * @param sendAndWait Send transaction, lookup next blocks for the response
 * @param onEveryBlock Repeat transaction for each non-empty block, stream responses
 */
class WorkerResponder[F[_]](
  val sendAndWait: SendAndWait[F],
  val onEveryBlock: RepeatOnEveryBlock[F]
)

object WorkerResponder {

  def make[F[_]: Parallel: Concurrent: Timer: Log, BC <: HList, B: TxsBlock](
    producer: BlockProducer.Aux[F, BC],
    machine: StateMachine[F],
    maxTries: Int = AwaitResponses.MaxBlocksTries
  )(
    implicit backoff: Backoff[EffectError],
    bs: ops.hlist.Selector[BC, BlockStream[F, B]]
  ): Resource[F, WorkerResponder[F]] =
    for {
      awaitResponses <- AwaitResponses.make(producer.command, machine, maxTries)
      sendAndWait = SendAndWait(producer, awaitResponses)
      onEveryBlock ← RepeatOnEveryBlock.make(producer.command, sendAndWait)
    } yield new WorkerResponder[F](
      sendAndWait,
      onEveryBlock
    )
}
