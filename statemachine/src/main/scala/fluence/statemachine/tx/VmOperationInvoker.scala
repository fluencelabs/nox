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

import cats.Monad
import cats.data.EitherT
import cats.effect.LiftIO
import fluence.statemachine.error
import fluence.statemachine.error.{PayloadParseError, StateMachineError, VmRuntimeError}
import fluence.vm.{VmError, WasmVm}
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.matching.Regex

/**
 * Invokes operations on provided VM.
 * TODO: Currently all error messages from VM converted to strings. Error hierarchy for the State machine required.
 *
 * @param vm VM instance used to make function calls and to retrieve state
 */
class VmOperationInvoker[F[_]: LiftIO](vm: WasmVm)(implicit F: Monad[F]) {
  private val FunctionWithModuleAndArgListPattern: Regex = "(\\w+)\\.(\\w+)\\(([\\w,]*)\\)".r
  private val FunctionWithoutModuleAndArgListPattern: Regex = "(\\w+)\\(([\\w,]*)\\)".r
  private val NonEmptyArgListPattern: Regex = "\\w+(,\\w+)*".r

  /**
   * Parses given text representation to the function call and invokes it using underlying VM.
   *
   * @param payload description of function invocation including function name and arguments
   * @return either successful invocation's result or failed invocation's error
   */
  def invoke(payload: String): EitherT[F, StateMachineError, Option[String]] = {
    for {
      parsedPayload <- payload match {
        case FunctionWithModuleAndArgListPattern(module, functionName, uncheckedArgList) =>
          EitherT.rightT((Some(module), functionName, uncheckedArgList))
        case FunctionWithoutModuleAndArgListPattern(functionName, uncheckedArgList) =>
          EitherT.rightT((None, functionName, uncheckedArgList))
        case _ => EitherT.leftT(VmOperationInvoker.wrongPayloadFormatError(payload))
      }
      (module, functionName, unparsedArgList: String) = parsedPayload
      parsedArgList <- unparsedArgList match {
        case NonEmptyArgListPattern(_) => EitherT.rightT(unparsedArgList.split(",").toList)
        case "" => EitherT.rightT(Nil)
        case _ => EitherT.leftT(VmOperationInvoker.wrongPayloadArgumentListFormatError(unparsedArgList))
      }
      vmInvocationResult <- vm
        .invoke(module, functionName, parsedArgList)
        .map(_.map(_.toString))
        .leftMap(VmOperationInvoker.convertToStateMachineError)

    } yield vmInvocationResult
  }

  /**
   * Obtains the current state hash of VM.
   *
   */
  def vmStateHash(): EitherT[F, StateMachineError, ByteVector] =
    vm.getVmState.leftMap(VmOperationInvoker.convertToStateMachineError)
}

object VmOperationInvoker {

  /**
   * Converts [[VmError]] to [[StateMachineError]]
   *
   * @param vmError error returned from VM
   */
  // todo handle different error types separately
  private def convertToStateMachineError(vmError: VmError): StateMachineError =
    VmRuntimeError(vmError.getClass.getSimpleName, vmError.getMessage, vmError)

  /**
   * Produces [[StateMachineError]] corresponding to payload that cannot be parsed to a function call.
   *
   * @param payload wrong payload
   */
  private def wrongPayloadFormatError(payload: String) =
    PayloadParseError("WrongPayloadFormat", s"Wrong payload format: $payload")

  /**
   * Produces [[StateMachineError]] corresponding to payload's argument list that cannot be parsed to
   * correct function arguments.
   *
   * @param unparsedArgList wrong payload argument list
   */
  private def wrongPayloadArgumentListFormatError(unparsedArgList: String) =
    error.PayloadParseError("WrongPayloadArgumentListFormat", s"Wrong payload arguments: $unparsedArgList")
}
