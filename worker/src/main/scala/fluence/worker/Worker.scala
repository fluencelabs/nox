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
import shapeless.HList

import scala.concurrent.duration.FiniteDuration
import scala.language.higherKinds

trait Worker[F[_]] {

  // TODO why should we need it?
  val appId: Long

  type Machine <: StateMachine[F]
  type Producer <: BlockProducer[F]

  def machine: Machine

  def producer: Producer

  def status(
    timeout: FiniteDuration
  )(implicit log: Log[F], timer: Timer[F], c: Concurrent[F], p: Parallel[F]): F[WorkerStatus]
}

object Worker {

  type Aux[F[_], C <: HList, B, PC <: HList] = Worker[F] {
    type Machine = StateMachine.Aux[F, C]

    type Producer = BlockProducer.Aux[F, B, PC]
  }

  type AuxM[F[_], C <: HList] = Worker[F] {
    type Machine = StateMachine.Aux[F, C]
  }

  type AuxP[F[_], B, C <: HList] = Worker[F] {
    type Producer = BlockProducer.Aux[F, B, C]
  }

  def apply[F[_]: Monad, C <: HList, B, PC <: HList](
    _appId: Long,
    _machine: StateMachine.Aux[F, C],
    _producer: BlockProducer.Aux[F, B, PC]
  ): Worker.Aux[F, C, B, PC] =
    new Worker[F] {
      override type Machine = StateMachine.Aux[F, C]
      override type Producer = BlockProducer.Aux[F, B, PC]

      override val appId: Long = _appId

      override val machine: Machine = _machine

      override val producer: Producer = _producer

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
