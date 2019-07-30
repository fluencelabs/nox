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

package fluence.vm.wasm.module

import cats.Monad
import cats.data.EitherT
import cats.effect.LiftIO
import fluence.vm.VmError.WasmVmError.{GetVmStateError, InvokeError}
import fluence.vm.VmError.{NoSuchFnError, VmMemoryError}
import fluence.vm.wasm.{WasmFunction, WasmModuleMemory}

import scala.language.higherKinds

/**
 * Wrapper of Wasm Module instance compiled by Asmble to Java class. Provides all functionality of Wasm modules
 * according to the Fluence protocol (invoke, parameter passing, hash computing). TODO: after removing alloc/
 * dealloc should be refactored to two modules types (extends the same trait): "master" (that has invoke method
 * and can routes call from user to "slaves") and "slave" (without invoke method that does only computation).
 *
 * @param allocateFunction a function used for allocation of a memory region for parameter passing
 * @param deallocateFunction a function used for deallocation of a memory region previously allocated
 *                          by allocateFunction
 * @param invokeFunction a function that represents main handler of Wasm module
 */
class MainModule(
  private val allocateFunction: WasmFunction,
  private val deallocateFunction: WasmFunction,
  private val invokeFunction: WasmFunction
) extends WasmModule {

  /**
   * Allocates a memory region in Wasm module of supplied size by allocateFunction.
   *
   * @param size a size of memory that need to be allocated
   */
  def allocate[F[_]: LiftIO: Monad](size: Int): EitherT[F, InvokeError, Int] =
    invokeWasmFunction(allocateFunction, Int.box(size) :: Nil)

  /**
   * Deallocates a previously allocated memory region in Wasm module by deallocateFunction.
   *
   * @param offset an address of the memory region to deallocate
   * @param size a size of memory region to deallocate
   */
  def deallocate[F[_]: LiftIO: Monad](offset: Int, size: Int): EitherT[F, InvokeError, Unit] =
    invokeWasmFunction(deallocateFunction, Int.box(offset) :: Int.box(size) :: Nil)
      .map(_ ⇒ ())

  /**
   * Invokes invokeFunction which exported from Wasm module with provided arguments.
   *
   * @param args arguments for invokeFunction
   */
  def invoke[F[_]: LiftIO: Monad](args: List[AnyRef]): EitherT[F, InvokeError, Int] =
    invokeWasmFunction(invokeFunction, args)

  /**
   * Reads [offset, offset+size) region from the module memory.
   *
   * @param offset an offset from which read should be started
   *  @param size bytes count to read
   */
  def readMemory[F[_]: Monad](offset: Int, size: Int): EitherT[F, VmMemoryError, Array[Byte]] =
    wasmMemory.readBytes(offset, size)

  /**
   * Writes array of bytes to module memory.
   *
   * @param offset an offset from which write should be started
   * @param injectedArray an array that should be written into the module memory
   */
  def writeMemory[F[_]: Monad](offset: Int, injectedArray: Array[Byte]): EitherT[F, VmMemoryError, Unit] =
    wasmMemory.writeBytes(offset, injectedArray)

  override def toString: String = name.getOrElse("<no-name>")
}
