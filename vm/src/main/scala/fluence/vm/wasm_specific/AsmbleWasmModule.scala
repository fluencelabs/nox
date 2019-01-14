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

package fluence.vm.wasm_specific

import java.lang.reflect.{Method, Modifier}
import java.nio.ByteBuffer

import asmble.run.jvm.Module.Compiled
import asmble.run.jvm.ScriptContext
import cats.data.EitherT
import cats.effect.LiftIO
import cats.Monad
import fluence.vm.VmError.WasmVmError.{ApplyError, InvokeError}
import fluence.vm.VmError.{InitializationError, InternalVmError, NoSuchFnError}

import scala.language.higherKinds
import scala.util.Try

/**
 * Wasm Module instance wrapper.
 *
 * @param name module name (can be empty)
 * @param instance wrapped instance of module
 * @param memory memory of this module
 */
class AsmbleWasmModule(
  private val name: Option[String],
  private val moduleState: WasmModuleState,
  private val instance: Any,
  private val allocateFunction: Option[WasmFunction],
  private val deallocateFunction: Option[WasmFunction],
  private val invokeFunction: Option[WasmFunction]
) {

  /**
    * Allocates a memory region in Wasm module of supplied size by allocateFunction.
    *
    * @param size size of memory that need to be allocated
    */
  def allocate[F[_]: LiftIO: Monad](size: Int): EitherT[F, InvokeError, AnyRef] =
    invokeWasmFunction(allocateFunction, size.asInstanceOf[AnyRef] :: Nil)

  /**
    * Deallocates a previously allocated memory region in Wasm module by deallocateFunction.
    *
    * @param offset address of the memory region to deallocate
    * @param size size of memory region to deallocate
    */
  def deallocate[F[_]: LiftIO: Monad](offset: Int, size: Int): EitherT[F, InvokeError, AnyRef] =
    invokeWasmFunction(deallocateFunction, offset.asInstanceOf[AnyRef] :: size.asInstanceOf[AnyRef] :: Nil)

  /**
    * Invokes invokeFunctionName which exported from Wasm module function with provided arguments.
    *
    * @param args arguments for invokeFunction
    */
  def invoke[F[_]: LiftIO: Monad](args: List[AnyRef]): EitherT[F, InvokeError, AnyRef] =
    invokeWasmFunction(invokeFunction, args)

  private def invokeWasmFunction[F[_]: LiftIO: Monad](
    wasmFunction: Option[WasmFunction],
    args: List[AnyRef]
  ): EitherT[F, InvokeError, AnyRef] =
    wasmFunction match {
      case Some(fn) => fn(instance, args)
      case _ =>
        EitherT.leftT(
          NoSuchFnError(s"Unable to find the function with name=$wasmFunction in module with name=$this")
        )
    }

  override def toString: String = name.getOrElse("<no-name>")
}

object AsmbleWasmModule {

  /**
   * Creates instance for specified module.
   *
   * @param moduleDescription a description of the module
   * @param scriptContext a context for the module operation
   */
  def apply(
    moduleDescription: Compiled,
    scriptContext: ScriptContext,
    allocationFunctionName: String,
    deallocationFunctionName: String,
    invokeFunctionName: String
  ): Either[ApplyError, AsmbleWasmModule] =
    for {

      moduleInstance <- Try(moduleDescription.instance(scriptContext)).toEither.left.map { e ⇒
        // todo method 'instance' must throw both an initialization error and a
        // Trap error, but now they can't be separated
        InitializationError(
          s"Unable to initialize module=${nameAsStr(moduleDescription.getName)}",
          Some(e)
        )
      }

      // getting memory field with reflection from module instance
      memory ← Try {
        // It's ok if a module doesn't have a memory
        val memoryMethod = Try(moduleInstance.getClass.getMethod("getMemory")).toOption
        memoryMethod.map(_.invoke(moduleInstance).asInstanceOf[ByteBuffer])
      }.toEither.left.map { e ⇒
        InternalVmError(
          s"Unable to getting memory from module=${nameAsStr(moduleDescription.getName)}",
          Some(e)
        )
      }

      // create a temporary map
      wasmExportFns: Map[String, Method] = moduleDescription
        .getCls
        .getDeclaredMethods
        .foldLeft(Map.empty[String, Method]) ((map, value) => value match {
          // choose only exported wasm functions
          case publicMethod if Modifier.isPublic(value.getModifiers) => map + (publicMethod.getName -> publicMethod)
          case _ => map
        }
      )

    } yield new AsmbleWasmModule(
      Option(moduleDescription.getName),
      WasmModuleState(memory),
      moduleInstance,
      constructWasmFunction(wasmExportFns, allocationFunctionName),
      constructWasmFunction(wasmExportFns, deallocationFunctionName),
      constructWasmFunction(wasmExportFns, invokeFunctionName)
  )

  private def constructWasmFunction(wasmFns: Map[String, Method], fnName: String) : Option[WasmFunction] =
    wasmFns.get(fnName).map(method => WasmFunction(method.getName, method))

  def nameAsStr(moduleName: Option[String]): String = moduleName.getOrElse("<no-name>")

  private def nameAsStr(moduleName: String): String = nameAsStr(Option(moduleName))

}
