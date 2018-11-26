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

package fluence.solver.vm

import cats.Monad
import cats.data.EitherT
import cats.effect.LiftIO
import cats.syntax.applicative._
import fluence.statemachine.tx.FunctionCallDescription
import fluence.statemachine.{Invoker, InvokerError, InvokerStateError}
import fluence.vm.VmError.WasmVmError.InvokeError
import fluence.vm.WasmVm
import scodec.bits.Bases.Alphabets.HexUppercase
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Invokes operations on provided VM.
 *
 * @param vm VM instance used to make function calls and to retrieve state
 */
class VmOperationInvoker[F[_]: LiftIO](vm: WasmVm)(implicit F: Monad[F]) extends Invoker[F] with slogging.LazyLogging {

  /**
   * Invokes the provided invocation description using the underlying VM.
   *
   * @param callDescription description of function call invocation including function name and arguments
   * @return either successful invocation's result or failed invocation's error
   */
  def invoke(callDescription: FunctionCallDescription): EitherT[F, InvokerError, Option[String]] = {
    val result = vm
      .invoke(callDescription.module, callDescription.functionName, callDescription.arg)
      .bimap(VmOperationInvoker.convertToInvokerError, _.map(ByteVector(_).toHex(HexUppercase)))
      .value

    EitherT(result)
  }

  /**
   * Obtains the current state hash of VM.
   *
   */
  def stateHash(): EitherT[F, InvokerStateError, ByteVector] =
    vm.getVmState.leftMap(_ => ???) //VmOperationInvoker.convertToSolverError)
}

object VmOperationInvoker {

  def convertToInvokerError(vmError: InvokeError): InvokerError = InvokerError(vmError.toString)

//  /**
//   * Converts [[VmError]] to [[SolverError]]
//   * TODO: handle different error types separately; possibly logging is required here.
//   *
//   * @param vmError error returned from VM
//   */
//  def convertToSolverError(vmError: VmError): SolverError = {
//    VmRuntimeError(vmError.getClass.getSimpleName, vmError.getMessage, vmError)
//  }
}
