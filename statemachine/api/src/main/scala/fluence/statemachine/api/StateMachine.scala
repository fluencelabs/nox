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
import fluence.statemachine.api.query.QueryResponse

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

  type Command

  def command: Command

  def query(path: String)(implicit log: Log[F]): EitherT[F, EffectError, QueryResponse]

  def status()(implicit log: Log[F]): EitherT[F, EffectError, StateMachineStatus]
}

object StateMachine {
  type Aux[F[_], C] = StateMachine[F] { type Command = C }
}
