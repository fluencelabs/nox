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
import fluence.vm.VmError.{InitializationError, NoSuchFnError, VmMemoryError}
import fluence.vm.wasm.{module, MemoryHasher, WasmFunction, WasmModuleMemory}

import scala.language.higherKinds
import scala.util.Try

/**
 * Wrapper of Wasm Module instance compiled by Asmble to Java class. Provides all functionality of Wasm modules
 * according to the Fluence protocol (invoke, parameter passing, hash computing). TODO: after removing alloc/
 * dealloc should be refactored to two modules types (extends the same trait): "master" (that has invoke method
 * and can routes call from user to "slaves") and "slave" (without invoke method that does only computation).
 *
 * @param name an optional module name (according to Wasm specification module name can be empty string (that is also
 *             "valid UTF-8") or even absent)
 * @param wasmMemory the memory of this module (please see comment in apply method to understand why it's optional now)
 * @param moduleInstance a instance of Wasm Module compiled by Asmble
 * @param allocateFunction a function used for allocation of a memory region for parameter passing
 * @param deallocateFunction a function used for deallocation of a memory region previously allocated
 *                          by allocateFunction
 * @param invokeFunction a function that represents main handler of Wasm module
 */
class EnvModule(
  private val module: WasmModule,
  private val spentGasFunction: WasmFunction,
  private val setSpentGasFunction: WasmFunction
) {

  /**
   * Allocates a memory region in Wasm module of supplied size by allocateFunction.
   */
  def getSpentGas[F[_]: LiftIO: Monad](): EitherT[F, InvokeError, Int] =
    module.invokeWasmFunction(spentGasFunction, Nil)

  /**
   * Deallocates a previously allocated memory region in Wasm module by deallocateFunction.
   *
   * @param offset an address of the memory region to deallocate
   * @param size a size of memory region to deallocate
   */
  def clearSpentGas[F[_]: LiftIO: Monad](): EitherT[F, InvokeError, Unit] =
    module
      .invokeWasmFunction(setSpentGasFunction, Int.box(0) :: Nil)
      .map(_ ⇒ ())

}

object EnvModule {

  /**
   * Creates instance for specified module.
   *
   * @param moduleDescription a Asmble description of the module
   * @param scriptContext a Asmble context for the module operation
   * @param memoryHasher a hasher used for compute hash if memory
   */
  def apply[F[_]: Monad](
    moduleDescription: Compiled,
    scriptContext: ScriptContext,
    memoryHasher: MemoryHasher.Builder[F],
    spentGasFunctionName: String,
    setSpentGasFunction: String
  ): EitherT[F, ApplyError, EnvModule] =
    for {
      module <- WasmModule(moduleDescription, scriptContext, memoryHasher)

      moduleMethods: Stream[WasmFunction] = moduleDescription.getCls.getDeclaredMethods.toStream
        .filter(method ⇒ Modifier.isPublic(method.getModifiers))
        .map(method ⇒ WasmFunction(method.getName, method))

      (spentGas: WasmFunction, setSpentGas: WasmFunction) <- EitherT.fromOption(
        moduleMethods
          .scanLeft((Option.empty[WasmFunction], Option.empty[WasmFunction])) {
            case (acc, m @ WasmFunction(`spentGasFunctionName`, _)) =>
              acc.copy(_1 = Some(m))
            case (acc, m @ WasmFunction(`setSpentGasFunction`, _)) =>
              acc.copy(_2 = Some(m))
            case (acc, _) =>
              acc
          }
          .collectFirst {
            case (Some(m1), Some(m2)) => (m1, m2)
          },
        NoSuchFnError(s"The env module must have function with names $spentGasFunctionName, $setSpentGasFunction"): ApplyError
      )

    } yield
      new EnvModule(
        module,
        spentGas,
        setSpentGas
      )

}
