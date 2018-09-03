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

package fluence.statemachine.util

/**
 * Constants used as `Info` for ABCI requests.
 */
object ClientInfoMessages {
  val UnknownClient: String = "Unknown client"
  val InvalidSignature: String = "Invalid signature"
  val DuplicatedTransaction: String = "Duplicated transaction"
  val SuccessfulTxResponse: String = ""

  val QueryStateIsNotReadyYet: String = "Query state is not ready yet"
  val RequestingCustomHeightIsForbidden: String = "Requesting custom height is forbidden"
  val InvalidQueryPath: String = "Invalid query path"
  val ResultIsNotReadyYet: String = "Result is not ready yet"
  val SuccessfulQueryResponse: String = ""
}
