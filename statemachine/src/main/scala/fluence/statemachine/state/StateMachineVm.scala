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

package fluence.statemachine.state

import cats.Monad
import cats.data.{EitherT, NonEmptyList}
import cats.effect.Sync
import fluence.log.Log
import fluence.statemachine.error.StateMachineError
import fluence.statemachine.vm.WasmVmOperationInvoker
import fluence.vm.WasmVm

import scala.language.higherKinds

object StateMachineVm {

  /**
   * Builds a VM instance used to perform function calls from the clients.
   *
   * @param moduleFiles module filenames with VM code
   */
  private[statemachine] def apply[F[_]: Sync: Log](
    moduleFiles: NonEmptyList[String]
  ): EitherT[F, StateMachineError, WasmVm] =
    WasmVm[F](moduleFiles).leftMap(WasmVmOperationInvoker.convertToStateMachineError)
}
