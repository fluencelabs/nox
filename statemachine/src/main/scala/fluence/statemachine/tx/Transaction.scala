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

import fluence.statemachine._

/**
 * Transaction header.
 * Contains important information about transaction and acts as it's unique identifier.
 *
 * @param client client identifier
 * @param session session identifier
 * @param order transaction counter in scope of session
 */
case class TransactionHeader(client: ClientId, session: SessionId, order: Long) {

  /**
   * Header describing transaction that required to be applied before the current transaction, if necessary.
   * The first (with index of 0) transaction of the session has no required transaction.
   *
   */
  def requiredTxHeader: Option[TransactionHeader] =
    if (order == 0) None else Some(TransactionHeader(client, session, order - 1))

  /**
   * Header describing a transaction that is the immediate transaction to be applied right after
   * the current transaction.
   *
   * Note that such transaction might be either queued (in case of out-of-order transaction sequence)
   * or not received yet (in normal case).
   *
   */
  def dependentTxHeader: TransactionHeader = TransactionHeader(client, session, order + 1)
}

/**
 * Transaction inside State machine. Contains [[header]] and [[payload]].
 * Does not include client signatures because State machine doesn't signatures after successful parsing.
 *
 * @param header transaction header
 * @param payload transaction payload describing operation that should be invoked by VM
 */
case class Transaction(header: TransactionHeader, payload: String) {
  def signString: String = s"${header.client}-${header.session}-${header.order}-$payload"
}

/**
 * [[Transaction]] together with client signature.
 * This is in fact deserialized representation of transaction got from Tendermint.
 *
 * @param tx transaction
 * @param signature signature used to prove the client's identity
 */
case class SignedTransaction(tx: Transaction, signature: String) {}
