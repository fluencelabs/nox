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

import fluence.vm.VmError.WasmVmError.{ApplyError, GetVmStateError, InvokeError}

import scala.util.control.NoStackTrace

/**
 * This type describes all errors appears in VM.
 */
sealed trait VmError extends NoStackTrace

abstract class VmErrorProxy(
  protected val message: String,
  protected val cause: Option[Throwable]
) extends Throwable(message, cause.orNull, true, false) with VmError

object VmError {

  /**
   * This type of error indicates some unexpected internal error has occurred in
   * the Virtual Machine.
   */
  case class InternalVmError(
    override val message: String,
    override val cause: Option[Throwable] = None
  ) extends VmErrorProxy(message, cause) with ApplyError with InvokeError with GetVmStateError

  /** Errors related to external WASM code. */
  sealed trait WasmError extends VmError

  /**
   * Indicates error when VM starts. It might be a problem with translation WASM
   * code to 'bytecode' or module instantiation. Module initialization is creation of
   * instance class that corresponds to WASM module.
   */
  case class InitializationError(
    override val message: String,
    override val cause: Option[Throwable] = None
  ) extends VmErrorProxy(message, cause) with WasmError with ApplyError

  /**
   * Indicates that some of the client input values are invalid. For example number
   * of types of argument is not correct or specified fn isn't exists.
   */
  sealed trait InvocationError extends WasmError with InvokeError

  /**
   * Indicates that arguments for fn invocation is not valid.
   */
  case class InvalidArgError(
    override val message: String,
    override val cause: Option[Throwable] = None
  ) extends VmErrorProxy(message, cause) with InvocationError

  /**
   * Indicates that fn with specified name wasn't found in a instance of VM.
   */
  case class NoSuchFnError(
    override val message: String,
    override val cause: Option[Throwable] = None
  ) extends VmErrorProxy(message, cause) with InvocationError with ApplyError

  /**
   * Indicates that WASM code execution was failed, some WASM instruction was
   * felled into the trap.
   */
  case class TrapError(
    override val message: String,
    override val cause: Some[Throwable]
  ) extends VmErrorProxy(message, cause) with WasmError with ApplyError with InvokeError

  /**
   * Contains errors for each [[fluence.vm.WasmVm]] public methods for reaching type-safe
   * working with produced errors. For example:
   * {{{
   *
   *  val vm: fluence.vm.WasmVm = ???
   *     vm.invoke[IO](None, "fnName", Seq()).leftMap {
   *       case InvalidArgError(message, cause) ⇒  ???
   *       case NoSuchFnError(message, cause) ⇒  ???
   *       case TrapError(message, cause) ⇒ ???
   *       // case InternalVmError(message, cause) ⇒ ???
   *     }
   *
   * }}}
   *
   *   Compiler checks that all of possible error types will be handled.
   *   If some of error types will not be handled compiler produce warning like this:
   *
   *   {{{
   *     Warning:(103, 50) match may not be exhaustive.
   *     It would fail on the following input: InternalVmError(_, _)
   *     vm.invoke[IO](None, "fnName", Seq()).leftMap {
   *   }}}
   *
   *   This mean that [[InternalVmError]] will not be handled.
   *
   */
  object WasmVmError {

    /** Error for [[fluence.vm.WasmVm:apply()]] method */
    sealed trait ApplyError extends VmError

    /** Error for [[fluence.vm.WasmVm:invoke()]] method */
    sealed trait InvokeError extends VmError

    /** Error for [[fluence.vm.WasmVm:getVmState()]] method */
    sealed trait GetVmStateError extends VmError

  }

}
