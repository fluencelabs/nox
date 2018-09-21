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
import fluence.statemachine.error.{StateMachineError, VmRuntimeError}
import fluence.vm.{VmError, WasmVm}
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Invokes operations on provided VM.
 *
 * @param vm VM instance used to make function calls and to retrieve state
 */
class VmOperationInvoker[F[_]: LiftIO](vm: WasmVm)(implicit F: Monad[F]) {

  /**
   * Invokes the provided invocation description using the underlying VM.
   *
   * @param callDescription description of function call invocation including function name and arguments
   * @return either successful invocation's result or failed invocation's error
   */
  def invoke(callDescription: FunctionCallDescription): EitherT[F, StateMachineError, Option[String]] =
    vm.invoke(callDescription.module, callDescription.functionName, callDescription.argList)
      .map(_.map(_.toString))
      .leftMap(VmOperationInvoker.convertToStateMachineError)

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
  def convertToStateMachineError(vmError: VmError): StateMachineError =
    VmRuntimeError(vmError.errorKind.getClass.getSimpleName, vmError.message, vmError)
}
