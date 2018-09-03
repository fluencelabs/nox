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
