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

package fluence.vm.error

/**
 * Base trait for errors occurred in Virtual machine.
 *
 * @param code short text code describing error, might be shown to the client
 * @param message detailed error message
 */
sealed abstract class VmError(val code: String, val message: String)

/**
 * Corresponds to errors occurred during VM initialization.
 *
 * @param code short text code describing error, might be shown to the client
 * @param message detailed error message
 */
case class InitializationError(override val code: String, override val message: String)
  extends VmError(code, message)

/**
 * Corresponds to errors occurred during VM function invocation.
 *
 * @param code short text code describing error, might be shown to the client
 * @param message detailed error message
 */
case class InvocationError(override val code: String, override val message: String)
  extends VmError(code, message)

/**
 * Corresponds to errors occurred during computing VM state hash.
 *
 * @param code short text code describing error, might be shown to the client
 * @param message detailed error message
 */
case class StateComputationError(override val code: String, override val message: String)
  extends VmError(code, message)
