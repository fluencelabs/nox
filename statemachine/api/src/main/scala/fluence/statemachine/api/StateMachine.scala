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

package fluence.statemachine.api

import cats.data.EitherT
import fluence.effects.EffectError
import fluence.log.Log
import fluence.statemachine.api.data.StateMachineStatus
import fluence.statemachine.api.query.QueryResponse
import shapeless._

import scala.language.higherKinds

/**
 * StateMachine represents the working, configured state machine.
 *
 * State machine takes commands from some interface (might be native, in case it's launched within the same JVM,
 * or might provide just ABCI) and replies on queries on its own.
 *
 * We have only two meaningful and stable query methods: query and status. In case some block producer needs
 * more, e.g. StateHash/BlockReceipt, it's just implementation details and should not affect the general picture.
 *
 * @tparam F Effect type
 */
trait StateMachine[F[_]] {
  self ⇒

  /**
   * Product (HList) of all types to access Command side of this state machine.
   */
  type Commands <: HList

  /**
   * Implementations for the command side
   */
  protected val commands: Commands

  /**
   * Access for a particular command. Usage: `machine.command[ConcreteCommandType]`
   *
   * @param cmd Selector; always available implicitly
   * @tparam C Command's type
   */
  final def command[C](implicit cmd: ops.hlist.Selector[Commands, C]): C = cmd(commands)

  def query(path: String)(implicit log: Log[F]): EitherT[F, EffectError, QueryResponse]

  def status()(implicit log: Log[F]): EitherT[F, EffectError, StateMachineStatus]

  final def extend[T](cmd: T): StateMachine.Aux[F, T :: Commands] = new StateMachine[F] {
    override type Commands = T :: self.Commands

    override protected val commands: Commands = cmd :: self.commands

    override def query(path: String)(implicit log: Log[F]): EitherT[F, EffectError, QueryResponse] =
      self.query(path)

    override def status()(implicit log: Log[F]): EitherT[F, EffectError, StateMachineStatus] =
      self.status()
  }
}

object StateMachine {
  type Aux[F[_], C] = StateMachine[F] { type Commands = C }

  abstract class ReadOnly[F[_]] extends StateMachine[F] {
    type Commands = HNil

    override protected val commands: Commands = HNil
  }
}
