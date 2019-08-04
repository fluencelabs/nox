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

import asmble.compile.jvm.MemoryBuffer
import asmble.run.jvm.Module.Compiled
import asmble.run.jvm.ScriptContext
import cats.Monad
import cats.data.EitherT
import fluence.vm.VmError.WasmVmError.{ApplyError, GetVmStateError}
import fluence.vm.VmError.InitializationError
import fluence.vm.utils.safelyRunThrowable
import fluence.vm.wasm.{module, MemoryHasher, WasmModuleMemory}

import scala.language.higherKinds

/**
 * Wrapper of Wasm Module instance compiled by Asmble to Java class. Provides all functionality of Wasm modules
 * according to the Fluence protocol (invoke, parameter passing, hash computing). TODO: after removing alloc/
 * dealloc should be refactored to two modules types (extends the same trait): "master" (that has invoke method
 * and can routes call from user to "slaves") and "slave" (without invoke method that does only computation).
 *
 * @param name an optional module name (according to Wasm specification module name can be empty string (that is also
 *             "valid UTF-8") or even absent)
 * @param wasmMemory the memory of this module (please see comment in apply method to understand why it's optional now)
 * @param instance a instance of Wasm Module compiled by Asmble
 */
class WasmModule(
  val name: Option[String],
  val wasmMemory: WasmModuleMemory,
  val instance: ModuleInstance
) {

  /**
   * Computes hash of all significant inner state of this Module. Now only memory is used for state hash computing;
   * other fields (such as Shadow stack, executed instruction counter, ...) should also be included after their
   * implementation.
   *
   */
  def computeStateHash[F[_]: Monad](): EitherT[F, GetVmStateError, Array[Byte]] =
    wasmMemory.computeMemoryHash()

  override def toString: String = name.getOrElse("<no-name>")
}

object WasmModule {

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
    memoryHasher: MemoryHasher.Builder[F]
  ): EitherT[F, ApplyError, module.WasmModule] =
    for {
      moduleInstance ← safelyRunThrowable(
        moduleDescription.instance(scriptContext),
        // TODO: method 'instance' can throw both an initialization error and a
        // Trap error, but now they can't be separated
        e ⇒ InitializationError(s"Unable to initialize module=${moduleDescription.getName}", Some(e))
      )

      // TODO: patch Asmble to create `getMemory` method in all cases
      memory ← safelyRunThrowable(
        {
          val getMemoryMethod = moduleInstance.getClass.getMethod("getMemory")
          getMemoryMethod.invoke(moduleInstance).asInstanceOf[MemoryBuffer]
        },
        e ⇒
          InitializationError(
            s"Unable to get memory from module=${Option(moduleDescription.getName).getOrElse("<no-name>")}",
            Some(e)
          ): ApplyError
      )

      moduleMemory ← WasmModuleMemory(memory, memoryHasher).leftMap(
        e ⇒
          InitializationError(
            s"Unable to instantiate WasmModuleMemory for module=${moduleDescription.getName}",
            Some(e)
          ): ApplyError
      )

    } yield
      new WasmModule(
        Option(moduleDescription.getName),
        moduleMemory,
        ModuleInstance(moduleInstance)
      )

}
