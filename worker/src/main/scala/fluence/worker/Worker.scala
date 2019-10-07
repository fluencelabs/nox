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

package fluence.worker

import cats.data.EitherT
import cats.effect.{Concurrent, Timer}
import cats.instances.either._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.{Monad, Parallel}
import fluence.bp.api.BlockProducer
import fluence.effects.EffectError
import fluence.log.Log
import fluence.statemachine.api.StateMachine
import shapeless._

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

/**
 * Worker is a composition of [[BlockProducer]] and corresponding [[StateMachine]].
 * Producer represents Command (Write) side, while Machine is a Read Model (Query side).
 * As for particular block producers and state machines we could have different specific services,
 * think of blockstore or consensus management, list of Companions is also introduced.
 * These Companions are built with access to internals of producer and machine, then acting as a facade for a
 * worker-associated logic and state changes.
 *
 * @param appId Application ID
 * @param companions Worker-specific companion services
 * @param machine State machine -- read model
 * @param producer Block producer -- write model
 * @tparam F Effect
 * @tparam CS Companions type
 */
abstract class Worker[F[_], CS <: HList](
// TODO why should we need it?
  val appId: Long,
  protected val companions: CS,
  val machine: StateMachine[F],
  val producer: BlockProducer[F]
) {
  self ⇒

  /**
   * Pick a companion of this Worker
   *
   * @tparam C Companion type
   */
  def companion[C](implicit c: ops.hlist.Selector[CS, C]): C = c(companions)

  /**
   * Fetch the status of this Worker as a whole
   *
   * @param timeout Status call fails if not collected within this duration
   * @return [[WorkerStatus]]
   */
  def status(
    timeout: FiniteDuration
  )(implicit log: Log[F], timer: Timer[F], c: Concurrent[F], p: Parallel[F]): F[WorkerStatus]

  /**
   * Map upon Worker's companions.
   * As we have type boundaries, map function is used instead of Functor implementation.
   *
   * @param fn Function to call on companions
   * @tparam CC Result companions type
   */
  def map[CC <: HList](fn: CS ⇒ CC): Worker[F, CC] = new Worker[F, CC](appId, fn(companions), machine, producer) {
    override def status(
      timeout: FiniteDuration
    )(implicit log: Log[F], timer: Timer[F], c: Concurrent[F], p: Parallel[F]): F[WorkerStatus] =
      self.status(timeout)
  }
}

object Worker {

  def apply[F[_]: Monad, C <: HList](
    appId: Long,
    machine: StateMachine[F],
    producer: BlockProducer[F],
    companions: C
  ): Worker[F, C] =
    new Worker[F, C](appId, companions, machine, producer) {

      def status(
        timeout: FiniteDuration
      )(implicit log: Log[F], timer: Timer[F], c: Concurrent[F], p: Parallel[F]): F[WorkerStatus] = {
        val sleep = Timer[F].sleep(timeout).as(s"Status timed out after $timeout")
        def ask[T](e: EitherT[F, EffectError, T]) = c.race(sleep, e.leftMap(_.toString).value).map(_.flatten)

        p.sequential(
          p.apply.map2(
            p.parallel(ask(machine.status())),
            p.parallel(ask(producer.status()))
          ) {
            case (Right(machineStatus), Right(producerStatus)) ⇒
              WorkerOperating(machineStatus, producerStatus)

            case (machineEither, producerEither) ⇒
              WorkerFailing(machineEither, producerEither)
          }
        )
      }

    }
}
