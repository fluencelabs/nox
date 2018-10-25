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

package fluence.statemachine.tx
import cats.Monad
import cats.data.EitherT
import com.github.jtendermint.jabci.types.Request.ValueCase
import fluence.statemachine.tree.{StoragePaths, TreeNode}
import fluence.statemachine.util.{ClientInfoMessages, Metrics}
import io.prometheus.client.Counter

import scala.language.higherKinds

/**
 * Checker for transactions received via `CheckTx` and `DeliverTx` and parsed.
 * Includes deduplication and checking whether tx' session is still active.
 *
 * @param method ABCI method that triggers this checker, used to configure logging and metric collection
 * @param state state against which checking is applied
 */
class TxStateDependentChecker[F[_]: Monad](val method: ValueCase, state: F[TreeNode]) {
  private val methodName = method.name().toLowerCase.replace("_", "")

  private val txCounter: Counter =
    Metrics.registerCounter(s"solver_${methodName}_count")
  private val txLatencyCounter: Counter =
    Metrics.registerCounter(s"solver_${methodName}_latency_sum")
  private val txValidationTimeCounter: Counter =
    Metrics.registerCounter(s"solver_${methodName}_validation_time_sum")

  /**
   * Checks whether given `tx` is unique and its state is active against provided [[state]].
   *
   * @param uncheckedTx parsed transaction
   * @return either checked transaction or error message
   */
  def check(uncheckedTx: Transaction): EitherT[F, String, Transaction] =
    for {
      currentState <- EitherT.right(state)
      deduplicatedTx <- EitherT.cond(
        !currentState
          .hasValue(StoragePaths.txStatusPath(uncheckedTx.header)),
        uncheckedTx,
        ClientInfoMessages.DuplicatedTransaction
      )
      checkedTx <- EitherT.cond(
        !currentState
          .getValue(StoragePaths.sessionSummaryPath(uncheckedTx.header))
          .flatMap(SessionSummary.fromStoreValue)
          .exists(_.status != Active),
        deduplicatedTx,
        ClientInfoMessages.SessionAlreadyClosed
      )
    } yield checkedTx

  /**
   * Collects transaction validation metrics.
   *
   * @param latency current tx latency
   * @param validationDuration tx validation duration
   */
  def collect(latency: Long, validationDuration: Long): Unit = {
    txCounter.inc()
    txLatencyCounter.inc(latency)
    txValidationTimeCounter.inc(validationDuration)
  }

}
