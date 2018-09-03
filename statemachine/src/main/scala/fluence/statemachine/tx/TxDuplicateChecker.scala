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
import cats.data.EitherT
import cats.syntax.functor._
import cats.{Eval, Monad}
import fluence.statemachine.tree.{StorageKeys, TreeNode}
import fluence.statemachine.util.ClientInfoMessages

import scala.language.higherKinds

/**
 * Uniqueness checker for transactions received via `CheckTx` and `DeliverTx` and parsed.
 *
 * @param state state for which deduplication is applied
 */
class TxDuplicateChecker[F[_]: Monad](state: F[TreeNode]) {

  /**
   * Checks whether given `tx` is unique against provided [[state]].
   *
   * @param tx parsed transaction
   * @return either deduplicated transaction or error message
   */
  def deduplicate(tx: Transaction): EitherT[F, String, Transaction] =
    EitherT(
      state.map(
        x => Either.cond(!x.hasValue(StorageKeys.statusKey(tx.header)), tx, ClientInfoMessages.DuplicatedTransaction)
      )
    )
}
