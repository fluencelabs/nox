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

package fluence.vm

import fluence.vm.VmError.VmErrorKind

import scala.util.control.NoStackTrace

/**
 * This type describes all errors appears in VM.
 */
case class VmError(
  message: String,
  causedBy: Option[Throwable],
  errorKind: VmErrorKind
) extends Throwable

object VmError {

  /** Root type for VM error kinds. */
  sealed trait VmErrorKind

  /**
   * This type of error indicates some unexpected internal error has occurred in
   * the Virtual Machine.
   */
  object InternalVmError extends VmErrorKind

  /** Errors related to external WASM code. */
  sealed trait WasmError extends VmErrorKind

  /**
   * Indicates error when VM starts. It might be a problem with translation WASM
   * code to 'bytecode' or module instantiation. Module initialization is creation of
   * instance class that corresponds to WASM module.
   */
  object InitializationError extends WasmError

  /**
   * Indicates that some of the client input values are invalid. For example number
   * of types of argument is not correct or specified fn isn't exists.
   */
  sealed trait InvocationError extends WasmError

  /**
   * Indicates that arguments for fn invocation is not valid.
   */
  object InvalidArgError extends InvocationError

  /**
   * Indicates that fn with specified name wasn't found in a instance of VM.
   */
  object NoSuchFnError extends InvocationError

  /**
   * Indicates that WASM code execution was failed, some WASM instruction was
   * felled into the trap.
   */
  object TrapError extends WasmError

  def apply(exception: Throwable, kind: VmErrorKind): VmError =
    VmError(exception.getMessage, Some(exception), kind)

  def apply(message: String, kind: VmErrorKind): VmError =
    VmError(message, None, kind)

}
