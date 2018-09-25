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
import fluence.statemachine.tx.{Transaction, TransactionHeader}

/**
 * Helper object for constructing [[TreeNode]] key paths used to
 * store State machine metadata and [[Transaction]] results.
 */
object StoragePaths {
  import StorageKeys._
  import fluence.statemachine.tree.TreeNode.{SelectorWildcardKey => WC}

  /**
   * Path used to store hash of the current state of underlying VM.
   */
  val VmStateHashPath: TreePath[StoreKey] = TreePath(List(MetadataRootKey, VmStateHashKey))

  /**
   * Path used to store current number of invoked transactions.
   */
  val TxCounterPath: TreePath[StoreKey] = TreePath(List(MetadataRootKey, TxCounterKey))

  /**
   * Selector path for searching all currently stored session summaries.
   */
  val SessionSummarySelector: TreePath[StoreKey] = TreePath(List(MetadataRootKey, WC, WC, SessionSummaryKey))

  /**
   * Path used to store a session summary.
   * If assigned, a serialized [[fluence.statemachine.tx.SessionSummary]] is used.
   *
   * @param txHeader transaction header
   */
  def sessionSummaryPath(txHeader: TransactionHeader): TreePath[StoreKey] =
    TreePath(List(MetadataRootKey, txHeader.client, txHeader.session, SessionSummaryKey))

  /**
   * Path used to store a transaction status.
   * If assigned, some of [[fluence.statemachine.tx.TransactionStatus]] values is used.
   *
   * @param txHeader transaction header
   */
  def txStatusPath(txHeader: TransactionHeader): TreePath[StoreKey] = txPathTemplate(txHeader, TxStatusKey)

  /**
   * Path used to store a payload for queued transactions.
   *
   * @param txHeader transaction header
   */
  def txPayloadPath(txHeader: TransactionHeader): TreePath[StoreKey] = txPathTemplate(txHeader, TxPayloadKey)

  /**
   * Path used to store the result (successful or failed) of transaction invocation.
   * If assigned, a serialized [[fluence.statemachine.tx.TxInvocationResult]] is used.
   *
   * @param txHeader transaction header
   */
  def txResultPath(txHeader: TransactionHeader): TreePath[StoreKey] = txPathTemplate(txHeader, TxResultKey)

  private def txPathTemplate(txHeader: TransactionHeader, postfix: String): TreePath[StoreKey] =
    TreePath(List(MetadataRootKey, txHeader.client, txHeader.session, txHeader.order.toString, postfix))
}

/**
 * Keys used to combine into state tree paths storing State machine metadata.
 */
object StorageKeys {
  val MetadataRootKey: StoreKey = "@meta"
  val VmStateHashKey: StoreKey = "@vmStateHash"
  val TxCounterKey: StoreKey = "@txCounter"
  val SessionSummaryKey: StoreKey = "@sessionSummary"
  val TxStatusKey: StoreKey = "status"
  val TxPayloadKey: StoreKey = "payload"
  val TxResultKey: StoreKey = "result"
}
