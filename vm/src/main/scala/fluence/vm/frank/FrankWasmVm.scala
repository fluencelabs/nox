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
import cats.effect.LiftIO
import fluence.vm.VmError.WasmVmError.{GetVmStateError, InvokeError}
import fluence.vm.{InvocationResult, WasmVm}
import scodec.bits.ByteVector
import fluence.vm.config.VmConfig

import scala.language.higherKinds

/**
 * Base implementation of [[WasmVm]].
 *
 * '''Note!!! This implementation isn't thread-safe. The provision of calls
 * linearization is the task of the caller side.'''
 */
class FrankWasmVm(
  vmRunnerInvoker: FrankAdapter,
  config: VmConfig
) extends WasmVm {

  override def invoke[F[_]: LiftIO: Monad](
    fnArgument: Array[Byte]
  ): EitherT[F, InvokeError, InvocationResult] = {
    val result = vmRunnerInvoker.invoke(fnArgument)
    EitherT.rightT[F, InvokeError](InvocationResult(result, 0))
  }

  override def getVmState[F[_]: LiftIO: Monad]: EitherT[F, GetVmStateError, ByteVector] = {
    val result = vmRunnerInvoker.getVmState()
    EitherT.rightT[F, GetVmStateError](ByteVector(result))
  }

  val expectsEth: Boolean = false
}
