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
import cats.syntax.functor._
import fluence.statemachine.error.{StateMachineError, VmRuntimeError}
import fluence.statemachine.util.{Metrics, TimeMeter}
import fluence.vm.{VmError, WasmVm}
import io.prometheus.client.Counter
import scodec.bits.Bases.Alphabets.HexUppercase
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Invokes operations on provided VM.
 *
 * @param vm VM instance used to make function calls and to retrieve state
 */
class VmOperationInvoker[F[_]: LiftIO](vm: WasmVm)(implicit F: Monad[F]) extends slogging.LazyLogging {

  private val vmInvokeCounter: Counter = Metrics.registerCounter("worker_vm_invoke_counter", "method")
  private val vmInvokeTimeCounter: Counter = Metrics.registerCounter("worker_vm_invoke_time_sum", "method")

  /**
   * Invokes the provided invocation description using the underlying VM.
   *
   * @param callDescription description of function call invocation including function name and arguments
   * @return either successful invocation's result or failed invocation's error
   */
  def invoke(callDescription: FunctionCallDescription): EitherT[F, StateMachineError, Option[String]] = {
    val invokeTimeMeter = TimeMeter()

    val result = for {
      invocationValue <- vm
        .invoke(callDescription.module, callDescription.arg)
        .bimap(VmOperationInvoker.convertToStateMachineError, _.map(ByteVector(_).toHex(HexUppercase)))
        .value

      invokeDuration = invokeTimeMeter.millisElapsed
      _ = logger.info("VmOperationInvoker duration={}", invokeDuration)

      _ = vmInvokeCounter.labels(callDescription.module.getOrElse("<no name module>")).inc()
      _ = vmInvokeTimeCounter.labels(callDescription.module.getOrElse("<no name module>")).inc(invokeDuration)
    } yield invocationValue

    EitherT(result)
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
   * TODO: handle different error types separately; possibly logging is required here.
   *
   * @param vmError error returned from VM
   */
  def convertToStateMachineError(vmError: VmError): StateMachineError =
    VmRuntimeError(vmError.getClass.getSimpleName, vmError.getMessage, vmError)
}
