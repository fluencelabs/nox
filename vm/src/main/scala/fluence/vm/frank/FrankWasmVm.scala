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

package fluence.vm.frank

import cats.Monad
import cats.data.EitherT
import cats.syntax.either._
import cats.effect.{IO, LiftIO}
import fluence.vm.{InvocationResult, WasmVm}
import scodec.bits.ByteVector
import fluence.vm.error.{InvocationError, StateComputationError}
import fluence.vm.frank.result.{RawInvocationResult, RawStateComputationResult}

import scala.language.higherKinds

/**
 * Base implementation of [[WasmVm]] based on the Wasmer execution environment.
 *
 * '''Note!!! This implementation isn't thread-safe. The provision of calls
 * linearization is the task of the caller side.'''
 */
class FrankWasmVm(
  vmRunnerInvoker: FrankAdapter
) extends WasmVm {

  override def invoke[F[_]: LiftIO: Monad](
    fnArgument: Array[Byte]
  ): EitherT[F, InvocationError, InvocationResult] = {
    println("start invoking")
    EitherT(
      IO(vmRunnerInvoker.invoke(fnArgument)).attempt
        .to[F]
    ).leftMap(e ⇒ InvocationError(s"Frank invocation failed by exception. Cause: ${e.getMessage}", Some(e)))
      .subflatMap {
        case RawInvocationResult(Some(err), _, _) ⇒ {
          println("RawInvocationResult error")
          InvocationError(s"Frank invocation failed. Cause: $err").asLeft[InvocationResult]
        }
        case RawInvocationResult(None, output, spentGas) ⇒
          println(s"RawInvocationResult success ${output.toString}")
          InvocationResult(output, spentGas).asRight[InvocationError]
      }
  }

  override def computeVmState[F[_]: LiftIO: Monad]: EitherT[F, StateComputationError, ByteVector] =
    EitherT(
      IO(vmRunnerInvoker.computeVmState()).attempt
        .to[F]
    ).leftMap(e ⇒ StateComputationError(s"Frank getting VM state failed. Cause: ${e.getMessage}", Some(e))).subflatMap {
      case RawStateComputationResult(Some(err), _) ⇒
        StateComputationError(s"Frank invocation failed. Cause: $err").asLeft[ByteVector]
      case RawStateComputationResult(None, state) ⇒
        ByteVector(state).asRight[StateComputationError]
    }

  val expectsEth: Boolean = false
}
