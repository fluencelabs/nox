/*
 * Copyright 2019 Fluence Labs Limited
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
import fluence.statemachine.util.HexCodec.hexToArray

import scala.language.higherKinds

sealed trait FunctionCallDescription

/**
 * Description of command that explicitly closes session by client.
 */
case class SmCloseSessionDescription() extends FunctionCallDescription

/**
 * Description of a Wasm function invocation with some argument.
 *
 * @param module VM module containing an invoked function
 * @param arg argument for an invoked function
 */
case class VmFunctionCallDescription(module: Option[String], arg: Array[Byte]) extends FunctionCallDescription

/**
 * Extractor for command that explicitly closes session.
 */
object SmCloseSession {
  val CloseSession = "@closeSession"

  def unapply(payload: String): Option[SmCloseSessionDescription] = payload match {
    case CloseSession => Some(SmCloseSessionDescription())
    case _ => None
  }

}

/**
 * Extractor of command that calls Wasm function.
 */
object VmFunctionCall {

  // ^ start of the line, needed to capture whole string, not just substring
  // (\w+)* optional module name
  // \((.*?)\) anything inside parentheses
  // $ end of the line, needed to capture whole string, not just substring
  private val payloadPattern = """^(\w+)*\((.*?)\)$""".r

  /**
   * Parses text payload in `[moduleName](arg)` format to a typed function call description.
   *
   * @param payload text representation of the function invocation
   */
  def unapply(payload: String): Option[VmFunctionCallDescription] = payload match {
    case payloadPattern(moduleName, rawFnArgs) =>
      hexToArray(rawFnArgs).toOption.map(arg => VmFunctionCallDescription(Option(moduleName), arg))

    case _ => None
  }

}
