/*
 * Copyright (C) 2017  Fluence Labs Limited
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
import fluence.statemachine.tree.TreeNode

/**
 * Uniqueness checker for transactions received via `CheckTx` and `DeliverTx` and parsed.
 *
 * @param state state for which deduplication is applied
 */
class TxDuplicateChecker(state: => TreeNode) {

  /**
   * Checks whether given `tx` is unique against provided [[state]].
   *
   * @param tx parsed transaction
   * @return either deduplicated transaction or error message
   */
  def deduplicate(tx: Transaction): Either[String, Transaction] =
    Either.cond(!state.hasValue(TxStorageKeys.statusKey(tx.header)), tx, "Duplicated transaction")
}
