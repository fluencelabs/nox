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

package fluence.statemachine.api.command

import cats.data.EitherT
import fluence.effects.EffectError
import fluence.log.Log
import fluence.statemachine.api.data.StateHash
import fluence.statemachine.api.tx.TxResponse

import scala.language.higherKinds

/**
 * Command side for changing the State machine internal state by passing and processing transactions, and commiting blocks.
 * Should be used to connect to a low-level block producer, e.g. a Tendermint node with ABCI interface.
 * This interface is not intended to be extended to cover more functionality (e.g. reconstructing Tendermint blocks).
 *
 */
trait TxProcessor[F[_]] {

  /**
   * Changes the State machine's state by applying a transaction
   *
   * @param txData Raw transaction data
   */
  def processTx(txData: Array[Byte])(implicit log: Log[F]): EitherT[F, EffectError, TxResponse]

  /**
   * Provides formal verification for the given transaction, not changing any state
   *
   * @param txData Raw transaction data
   */
  def checkTx(txData: Array[Byte])(implicit log: Log[F]): EitherT[F, EffectError, TxResponse]

  /**
   * Finish block handling, update state hash
   */
  def commit()(implicit log: Log[F]): EitherT[F, EffectError, StateHash]

}
