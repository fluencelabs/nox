/*
 * Copyright (C) 2018  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
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
