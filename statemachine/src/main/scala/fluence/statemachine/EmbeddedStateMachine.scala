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

package fluence.statemachine

import cats.Functor
import cats.data.EitherT
import cats.syntax.functor._
import fluence.effects.EffectError
import fluence.log.Log
import fluence.statemachine.api.StateMachine
import fluence.statemachine.api.command._
import fluence.statemachine.api.data.{StateHash, StateMachineStatus}
import fluence.statemachine.api.query.QueryResponse
import fluence.statemachine.api.tx.TxResponse
import fluence.statemachine.state.StateService
import shapeless._

import scala.language.higherKinds

object EmbeddedStateMachine {

  def apply[F[_]: Functor](
    stateService: StateService[F],
    initialStatus: StateMachineStatus
  ): StateMachine.Aux[F, HashesBus[F] :: TxProcessor[F] :: HNil] =
    new StateMachine.ReadOnly[F] {
      override def query(path: String)(implicit log: Log[F]): EitherT[F, EffectError, QueryResponse] =
        EitherT right stateService.query(path)

      override def status()(implicit log: Log[F]): EitherT[F, EffectError, StateMachineStatus] =
        EitherT right stateService.stateHash.map(h ⇒ initialStatus.copy(stateHash = h))
    }.extend[TxProcessor[F]](
        new TxProcessor[F] {
          override def processTx(txData: Array[Byte])(implicit log: Log[F]): EitherT[F, EffectError, TxResponse] =
            EitherT right stateService.deliverTx(txData)

          override def checkTx(txData: Array[Byte])(implicit log: Log[F]): EitherT[F, EffectError, TxResponse] =
            EitherT right stateService.checkTx(txData)

          override def commit()(implicit log: Log[F]): EitherT[F, EffectError, StateHash] =
            EitherT right stateService.commit
        }
      )
      .extend[HashesBus[F]](
        stateService.hashesBus
      )

}
