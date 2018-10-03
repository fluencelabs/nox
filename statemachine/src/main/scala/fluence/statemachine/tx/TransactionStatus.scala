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
import fluence.statemachine.StoreValue

/**
 * Transaction status describing whether some transaction is already invoked (successfully or not)
 * or still queued for the invocation.
 *
 * @param storeValue status representation for storing it in the state tree
 * @param sessionStatus new status for session to which the transaction belongs
 * @param allowDependentTxInvocation whether the next, dependent, transaction might be scheduled for invocation
 */
sealed abstract class TransactionStatus(
  val storeValue: StoreValue,
  val sessionStatus: SessionStatus,
  val allowDependentTxInvocation: Boolean
)

object TransactionStatus {

  /**
   * Status corresponding to a queued transaction that was checked but not ready to be invoked.
   */
  case object Queued extends TransactionStatus("queued", Active, false)

  /**
   * Status corresponding to a transaction that was successfully invoked.
   */
  case object Success extends TransactionStatus("success", Active, true)

  /**
   * Status corresponding to a transaction that was failed during its invocation.
   */
  case object Error extends TransactionStatus("error", Failed, false)

  /**
   * Status corresponding to a successfully invoked session-closing transaction.
   */
  case object SessionClosed extends TransactionStatus("sessionClosed", ExplicitlyClosed, false)

  private val StatusList = List(Queued, Success, Error, SessionClosed)

  /**
   * Deserializes the transaction status from the given [[StoreValue]].
   *
   * @param storeValue serialized transaction status
   */
  def fromStoreValue(storeValue: StoreValue): Option[TransactionStatus] = StatusList.find(_.storeValue == storeValue)

}
