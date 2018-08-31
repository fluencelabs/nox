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

package fluence.statemachine.tx
import fluence.statemachine.StoreValue
import io.circe.generic.auto._
import io.circe.syntax._

/**
 * The result of invocation attempt of some [[Transaction]].
 */
sealed abstract class TxInvocationResult {

  /**
   * Obtains representation to store in the state tree.
   */
  def toStoreValue: StoreValue = this.asJson.noSpaces
}

/**
 * The result corresponding to a successful invocation with returned value.
 *
 * @param value returned value
 */
case class Computed(value: String) extends TxInvocationResult

/**
 * The result corresponding to a successful invocation without returned value.
 */
case object Empty extends TxInvocationResult

/**
 * The result corresponding to a failed invocation.
 *
 * @param code short text code describing error
 * @param message detailed error message
 */
case class Error(code: String, message: String) extends TxInvocationResult
