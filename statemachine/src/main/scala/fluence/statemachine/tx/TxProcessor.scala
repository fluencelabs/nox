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

import fluence.statemachine.state.MutableStateTree
import fluence.statemachine.tx.TxStorageKeys.{errorMessageKey, payloadKey, resultKey, statusKey}
import slogging.LazyLogging

/**
 * Performs a series of actions with successfully validated (successfully parsed and unique) transaction received
 * in `DeliverTx` request.
 *
 * Any transaction received by [[TxProcessor]] affects the application state since the processing start,
 * even if transaction would be rejected by VM or it's invocation would be failed.
 * It also means that Consensus tree root's hash (interpreted as `app_hash` on `Commit`) is almost certainly changed
 * (except rare cases of collisions).
 *
 * Note that `DeliverTx` processing is synchronous, `DeliverTxResponse` sending is performed only after
 * transaction appending, invocation and even invocation of dependent, previously queued transactions.
 *
 * @param consensusState Consensus state, affected by every newly appended transaction
 */
class TxProcessor(consensusState: MutableStateTree) extends LazyLogging {
  private val txInvoker = new TxInvoker

  /**
   * Enqueues given `tx` and tries to apply it immediately.
   * Otherwise this transaction remains queued and not applied until required transaction becomes successfully applied.
   *
   * As soon as any transaction becomes successfully applied it tries to apply the dependent transaction if it is
   * queued.
   *
   * @param tx transaction ready to be processed
   */
  def appendTx(tx: Transaction): Unit = {
    logger.debug("Appended tx: {}", tx)
    enqueueTx(tx)
    if (tx.header.requiredTxHeader.forall(header => getStatus(header).contains(TransactionStatus.Success))) {
      fireTxApplication(tx.header)
    }
  }

  /**
   * Triggers the application of a transaction stored by given header (if actually stored).
   * In case of success recursively triggers the application of dependent transactions that might be queued
   * (if received out of order previously).
   *
   * @param txHeader header of the transaction to be applied
   */
  private def fireTxApplication(txHeader: TransactionHeader): Unit = {
    getStoredPayload(txHeader)
      .foreach(payload => {
        val tx = Transaction(txHeader, payload)
        applyTx(tx)
        dequeueTx(tx)
        fireTxApplication(txHeader.dependentTxHeader)
      })
  }

  /**
   * Effectively applies given transaction and appends it's results to Consensus state.
   *
   * @param tx transaction ready to be applied (by the invocation in VM)
   */
  private def applyTx(tx: Transaction): Unit = {
    txInvoker.invokeTx(tx.payload) match {
      case Left(errorMessage) => putErrorMessage(tx, errorMessage)
      case Right(result) => putResult(tx, result)
    }
  }

  private def enqueueTx(tx: Transaction): Unit = {
    consensusState.putValue(payloadKey(tx.header), tx.payload)
    consensusState.putValue(statusKey(tx.header), TransactionStatus.Queued)
  }

  private def dequeueTx(tx: Transaction): Unit = {
    consensusState.removeValue(payloadKey(tx.header))
  }

  private def getStatus(txHeader: TransactionHeader): Option[String] =
    consensusState.getRoot.getValue(statusKey(txHeader))

  private def getStoredPayload(txHeader: TransactionHeader): Option[String] =
    consensusState.getRoot.getValue(payloadKey(txHeader))

  private def putResult(tx: Transaction, result: String): Unit = {
    consensusState.putValue(resultKey(tx.header), result)
    consensusState.putValue(statusKey(tx.header), TransactionStatus.Success)
  }

  private def putErrorMessage(tx: Transaction, errorMessage: String): Unit = {
    consensusState.putValue(errorMessageKey(tx.header), errorMessage)
    consensusState.putValue(statusKey(tx.header), TransactionStatus.Error)
  }

}
