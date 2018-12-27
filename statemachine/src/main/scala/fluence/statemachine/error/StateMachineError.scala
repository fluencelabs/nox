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
 * @param code short text code describing error, might be shown to the client
 * @param message detailed error message
 * @param causedBy caught [[Throwable]], if any
 */
sealed abstract class StateMachineError(val code: String, val message: String, val causedBy: Option[Throwable])

/**
 * Corresponds to errors occurred during payload parsing before invoking VM function.
 *
 * @param code short text code describing error, might be shown to the client
 * @param message detailed error message
 */
case class PayloadParseError(override val code: String, override val message: String)
    extends StateMachineError(code, message, None)

/**
 * Corresponds to errors occurred during VM function invocation inside VM.
 *
 * @param code short text code describing error, might be shown to the client
 * @param message detailed error message
 * @param vmError caught [[VmError]]
 */
case class VmRuntimeError(override val code: String, override val message: String, vmError: VmError)
    extends StateMachineError(code, message, Some(vmError))

/**
 * Corresponds to errors occurred during State machine config loading.
 *
 * @param message detailed error message
 */
case class ConfigLoadingError(override val message: String)
    extends StateMachineError("ConfigLoadingError", message, None)

/**
 * Corresponds to errors occurred during looking for VM module files before passing them to VM.
 *
 * @param message detailed error message
 * @param throwable caught [[Throwable]]
 */
case class VmModuleLocationError(override val message: String, throwable: Option[Throwable] = None)
    extends StateMachineError("VmModuleLocationError", message, throwable)
