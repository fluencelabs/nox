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
import io.circe.generic.auto._
import io.circe.parser.{parse => parseJson}
import io.circe.syntax._

/**
 * Summary for the client session, containing the information about the session lifecycle.
 * This information is used by the State machine ifself (for transaction checking, session expiration, etc.)
 * and by the client querying it from the state tree.
 *
 * @param status [[SessionStatus]] describing this session (a particular phase of the session lifecycle)
 * @param invokedTxsCount number of transactions of this session invoked (regardless successfully or not)
 *                        by the State machine
 * @param lastTxCounter the global invoked tx counter value of the State machine for the latest invoked transaction
 *                      of this session
 */
case class SessionSummary(status: SessionStatus, invokedTxsCount: Long, lastTxCounter: Long) {

  /**
   * Serializes the summary to [[StoreValue]] in order to store it in the state tree.
   */
  def toStoreValue: StoreValue = this.asJson.noSpaces
}

object SessionSummary {

  /**
   * Deserializes the summary from the given [[StoreValue]].
   * TODO: resolve unsafe parsing here, by changing result type.
   *
   * @param value serialized summary
   */
  def fromStoreValue(value: StoreValue): Option[SessionSummary] =
    (for {
      parsedJson <- parseJson(value)
      summary <- parsedJson.as[SessionSummary]
    } yield summary).toOption
}

/**
 * Status describing the current phase of the sesion lifecycle.
 */
sealed abstract class SessionStatus

/**
 * Status corresponding to sessions that active, i. e. could potentially invoke new transactions.
 */
case object Active extends SessionStatus

/**
 * Status corresponding to sessions closed by any reason and unable to invoke any transactions.
 */
sealed abstract class Closed extends SessionStatus

/**
 * Status corresponding to sessions explicitly closed by their clients via sending `@closeSession` transaction.
 */
case object ExplicitlyClosed extends Closed

/**
 * Status corresponding to sessions explicitly closed by their clients via sending `@closeSession` transaction.
 */
case object Failed extends Closed

/**
 * Status corresponding to sessions neither closed by their clients no failed and later considered `expired`
 * after some inactivity period.
 */
case object Expired extends Closed
