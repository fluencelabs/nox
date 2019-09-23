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

package fluence.worker.api

import fluence.bp.api.BlockProducer
import fluence.statemachine.api.StateMachine
import shapeless.HList

import scala.language.higherKinds

trait Worker[F[_]] {

  val appId: Long

  type Machine <: StateMachine[F]
  type Producer <: BlockProducer[F]

  def machine: Machine

  def producer: Producer

}

object Worker {

  type Aux[F[_], C <: HList, B] = Worker[F] {
    type Machine = StateMachine.Aux[F, C]

    type Producer = BlockProducer.Aux[F, B]
  }

  type AuxM[F[_], C <: HList] = Worker[F] {
    type Machine = StateMachine.Aux[F, C]
  }

  type AuxP[F[_], B] = Worker[F] {
    type Producer = BlockProducer.Aux[F, B]
  }

  def apply[F[_], C <: HList, B](
    _appId: Long,
    _machine: StateMachine.Aux[F, C],
    _producer: BlockProducer.Aux[F, B]
  ): Worker.Aux[F, C, B] =
    new Worker[F] {
      override type Machine = StateMachine.Aux[F, C]
      override type Producer = BlockProducer.Aux[F, B]

      override val appId: Long = _appId

      override val machine: Machine = _machine

      override val producer: Producer = _producer
    }
}
