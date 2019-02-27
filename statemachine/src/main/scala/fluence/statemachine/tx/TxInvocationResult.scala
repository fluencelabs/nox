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
case class Error(code: String, message: String, cause: Option[String]) extends TxInvocationResult
