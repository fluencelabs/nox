/*
 * Copyright (C) 2017  Fluence Labs Limited
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
) extends NoStackTrace

object VmError {

  /** Root type for VM error kinds. */
  sealed trait VmErrorKind

  /**
   * This type of error indicates some unexpected internal error has occurred in
   * the Virtual Machine.
   */
  // todo this type is not used yet, because we should support it in ''Asmble''
  object Internal extends VmErrorKind

  /**
   * This type of error indicates that an error occurred while preparing or
   * executing some WASM code.
   */
  sealed trait Execution extends VmErrorKind

  /**
   * Indicates error when VM starts: compiling WASM to JVM code,
   * modules instantiation and so on. Module initialization is creation of
   * instance class that corresponds to WASM module.
   */
  object Initialization extends Execution

  /**
   * Indicates that some of the client input values are invalid.
   */
  object Validation extends Execution

  /**
   * Indicates that some error occurs when Wasm code was executed.
   */
  object Runtime extends Execution

  def apply(exception: Throwable, kind: VmErrorKind): VmError =
    VmError(exception.getMessage, Some(exception), kind)

  def validation(msg: String, cause: Option[Throwable] = None): VmError =
    VmError(msg, None, Validation)

  def internalErr(exception: Throwable): VmError =
    VmError(exception.getMessage, Some(exception), Internal)

  def internalErr(msg: String, cause: Option[Throwable] = None): VmError =
    VmError(msg, cause, Internal)

}
