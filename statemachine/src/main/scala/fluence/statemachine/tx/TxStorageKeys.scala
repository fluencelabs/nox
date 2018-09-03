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
import fluence.statemachine.StoreKey
import fluence.statemachine.tree.TreePath

/**
 * Helper object for constructing [[fluence.statemachine.tree.TreeNode]] keys used to
 * store [[Transaction]] metadata and results.
 */
object TxStorageKeys {

  /**
   * Key used to store a transaction status. If assigned, some of [[TransactionStatus]] values is used.
   *
   * @param txHeader transaction header
   */
  def statusKey(txHeader: TransactionHeader): TreePath[StoreKey] = filledKeyTemplate(txHeader, "status")

  /**
   * Key used to store a payload for queued transactions.
   *
   * @param txHeader transaction header
   */
  def payloadKey(txHeader: TransactionHeader): TreePath[StoreKey] = filledKeyTemplate(txHeader, "payload")

  /**
   * Keys used to store a successful transaction's result.
   *
   * @param txHeader transaction header
   */
  def resultKey(txHeader: TransactionHeader): TreePath[StoreKey] = filledKeyTemplate(txHeader, "result")

  /**
   * Keys used to store a failed transaction's error message.
   *
   * @param txHeader transaction header
   */
  def errorMessageKey(txHeader: TransactionHeader): TreePath[StoreKey] = filledKeyTemplate(txHeader, "error")

  private def filledKeyTemplate(txHeader: TransactionHeader, postfix: String): TreePath[StoreKey] =
    TreePath(List("@meta", txHeader.client, txHeader.session, txHeader.order.toString, postfix))
}

/**
 * Contains possible statuses that stored for transaction that is queued, applying or applied.
 */
object TransactionStatus {
  val Queued: String = "queued"
  val Success: String = "success"
  val Error: String = "error"
}
