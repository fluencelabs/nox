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

package fluence.statemachine.tree
import fluence.statemachine.StoreKey
import fluence.statemachine.tx.{Transaction, TransactionHeader, TransactionStatus}

/**
 * Helper object for constructing [[TreeNode]] keys used to
 * store State machine metadata and [[Transaction]] results.
 */
object StorageKeys {

  /**
   * Key used to store hash of the current state of underlying VM.
   */
  val vmStateHashKey: TreePath[StoreKey] = TreePath(List("@meta", "@vm_state_hash"))

  /**
   * TODO:
   */
  val txCounterKey: TreePath[StoreKey] = TreePath(List("@meta", "@tx_counter"))

  /**
   * TODO:
   */
  val sessionStatusKeyTemplate: TreePath[StoreKey] = TreePath(List("@meta", "*", "*", "@session_status"))

  /**
   * TODO:
   *
   * @param txHeader
   */
  def sessionStatusKey(txHeader: TransactionHeader): TreePath[StoreKey] =
    TreePath(List("@meta", txHeader.client, txHeader.session, "@session_status"))

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
   * Keys used to store the result (successful or failed) of transaction invocation.
   *
   * @param txHeader transaction header
   */
  def resultKey(txHeader: TransactionHeader): TreePath[StoreKey] = filledKeyTemplate(txHeader, "result")

  private def filledKeyTemplate(txHeader: TransactionHeader, postfix: String): TreePath[StoreKey] =
    TreePath(List("@meta", txHeader.client, txHeader.session, txHeader.order.toString, postfix))
}
