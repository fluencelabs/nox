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

import java.lang.reflect.Modifier

import asmble.run.jvm.Module.Compiled
import asmble.run.jvm.ScriptContext
import cats.Monad
import cats.data.EitherT
import cats.effect.LiftIO
import fluence.vm.VmError.WasmVmError.{ApplyError, GetVmStateError, InvokeError}
import fluence.vm.VmError.{NoSuchFnError, VmMemoryError}
import fluence.vm.wasm._

import scala.language.higherKinds

/**
 * Wrapper for a Main module instance compiled by Asmble to Java class (please find more info about module types in
 * WasmModule docs). Provides all functionality of Main Wasm modules according to the Fluence protocol (invoke,
 * parameter passing, hash computing).
 *
 * @param module a wasm module used as a container for the module memory and instance
 * @param allocateFunction a function used for allocation of a memory region for parameter passing
 * @param deallocateFunction a function used for deallocation of a memory region previously allocated
 *                          by allocateFunction
 * @param invokeFunction a function that represents main handler of Wasm module
 */
class MainWasmModule(
  private val module: WasmModule,
  private val allocateFunction: WasmFunction,
  private val deallocateFunction: WasmFunction,
  private val invokeFunction: WasmFunction
) {

  def computeStateHash[F[_]: Monad](): EitherT[F, GetVmStateError, Array[Byte]] =
    module.computeStateHash()

  /**
   * Allocates a memory region in Wasm module of supplied size by allocateFunction.
   *
   * @param size a size of memory that need to be allocated
   */
  def allocate[F[_]: LiftIO: Monad](size: Int): EitherT[F, InvokeError, Int] =
    allocateFunction(module.instance, Int.box(size) :: Nil).map(_.get.intValue())

  /**
   * Deallocates a previously allocated memory region in Wasm module by deallocateFunction.
   *
   * @param offset an address of the memory region to deallocate
   * @param size a size of memory region to deallocate
   */
  def deallocate[F[_]: LiftIO: Monad](offset: Int, size: Int): EitherT[F, InvokeError, Unit] =
    deallocateFunction(module.instance, Int.box(offset) :: Int.box(size) :: Nil).map(_ ⇒ ())

  /**
   * Invokes invokeFunction which exported from Wasm module with provided arguments.
   *
   * @param args arguments for invokeFunction
   */
  def invoke[F[_]: LiftIO: Monad](args: List[AnyRef]): EitherT[F, InvokeError, Int] =
    invokeFunction(module.instance, args).map(_.get.intValue())

  /**
   * Reads [offset, offset+size) region from the module memory.
   *
   * @param offset an offset from which read should be started
   *  @param size bytes count to read
   */
  def readMemory[F[_]: Monad](offset: Int, size: Int): EitherT[F, VmMemoryError, Array[Byte]] =
    module.wasmMemory.readBytes(offset, size)

  /**
   * Writes array of bytes to module memory.
   *
   * @param offset an offset from which write should be started
   * @param injectedArray an array that should be written into the module memory
   */
  def writeMemory[F[_]: Monad](offset: Int, injectedArray: Array[Byte]): EitherT[F, VmMemoryError, Unit] =
    module.wasmMemory.writeBytes(offset, injectedArray)

  override def toString: String = module.toString
}

object MainWasmModule {

  /**
   * Creates instance for specified module.
   *
   * @param moduleDescription a Asmble description of the module
   * @param scriptContext a Asmble context for the module operation
   * @param memoryHasher a hasher used for compute hash if memory
   * @param allocationFunctionName a name of function used for allocation of a memory region for parameter passing
   * @param deallocationFunctionName a name of function used for deallocation of a memory region previously allocated
   *                                 by allocateFunction
   * @param invokeFunctionName a name of function that represents main handler of Wasm module
   */
  def apply[F[_]: Monad](
    moduleDescription: Compiled,
    scriptContext: ScriptContext,
    memoryHasher: MemoryHasher.Builder[F],
    allocationFunctionName: String,
    deallocationFunctionName: String,
    invokeFunctionName: String
  ): EitherT[F, ApplyError, MainWasmModule] =
    for {
      module <- WasmModule(moduleDescription, scriptContext, memoryHasher)

      moduleMethods: Stream[WasmFunction] = moduleDescription.getCls.getDeclaredMethods.toStream
        .filter(method ⇒ Modifier.isPublic(method.getModifiers))
        .map(method ⇒ WasmFunction(method.getName, method))

      (allocMethod, deallocMethod, invokeMethod) <- EitherT.fromOption(
        moduleMethods
          .scanLeft((Option.empty[WasmFunction], Option.empty[WasmFunction], Option.empty[WasmFunction])) {
            case (acc, m @ WasmFunction(`allocationFunctionName`, _)) =>
              acc.copy(_1 = Some(m))
            case (acc, m @ WasmFunction(`deallocationFunctionName`, _)) =>
              acc.copy(_2 = Some(m))
            case (acc, m @ WasmFunction(`invokeFunctionName`, _)) =>
              acc.copy(_3 = Some(m))
            case (acc, _) =>
              acc
          }
          .collectFirst {
            case (Some(allocMethod), Some(deallocMethod), Some(invokeMethod)) =>
              (allocMethod, deallocMethod, invokeMethod)
          },
        NoSuchFnError(
          s"The main module must have functions with names $allocationFunctionName, $deallocationFunctionName, $invokeFunctionName"
        ): ApplyError
      )

    } yield
      new MainWasmModule(
        module,
        allocMethod,
        deallocMethod,
        invokeMethod
      )

}
