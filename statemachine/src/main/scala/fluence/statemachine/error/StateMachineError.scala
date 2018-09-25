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

package fluence.statemachine.error
import fluence.vm.VmError

/**
 * Base trait for errors occurred in State machine.
 *
 */
sealed trait StateMachineError {

  val code: String // short text code describing error, might be shown to the client

  val message: String // detailed error message

  val causedBy: Option[Throwable] // caught [[Throwable]], if any
}

/**
 * Corresponds to errors occurred during payload parsing before invoking VM function.
 */
case class PayloadParseError(override val code: String, override val message: String) extends StateMachineError {
  override val causedBy: Option[Throwable] = None
}

/**
 * Corresponds to errors occurred during VM function invocation inside VM.
 */
case class VmRuntimeError(override val code: String, override val message: String, vmError: VmError)
    extends StateMachineError {
  override val causedBy: Option[Throwable] = Some(vmError)
}

/**
 * Corresponds to errors occurred during State machine config loading.
 */
case class ConfigLoadingError(override val code: String, override val message: String) extends StateMachineError {
  override val causedBy: Option[Throwable] = None
}
