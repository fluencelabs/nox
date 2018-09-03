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
  private def convertToStateMachineError(vmError: VmError): StateMachineError =
    VmRuntimeError(vmError.errorKind.getClass.getSimpleName, vmError.message, vmError)

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
