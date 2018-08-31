/*
 * Copyright (C) 2018  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
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
