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
import fluence.statemachine.tree.{StorageKeys, TreeNode}
import fluence.statemachine.util.ClientInfoMessages

import scala.language.higherKinds

/**
 * Checker for transactions received via `CheckTx` and `DeliverTx` and parsed.
 * Includes deduplication and checking whether tx' session is still active.
 *
 * @param state state against which checking is applied
 */
class TxStateDependentChecker[F[_]: Monad](state: F[TreeNode]) {

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
          .hasValue(StorageKeys.statusKey(uncheckedTx.header)),
        uncheckedTx,
        ClientInfoMessages.DuplicatedTransaction
      )
      checkedTx <- EitherT.cond(
        !currentState
          .getValue(StorageKeys.sessionSummaryKey(uncheckedTx.header))
          .flatMap(SessionSummary.fromStoreValue)
          .exists(_.status != Active),
        deduplicatedTx,
        ClientInfoMessages.SessionAlreadyClosed
      )
    } yield checkedTx
}
