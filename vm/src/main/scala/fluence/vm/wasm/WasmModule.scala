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

package fluence.vm.wasm

import java.lang.reflect.{Method, Modifier}
import java.nio.ByteBuffer

import asmble.run.jvm.Module.Compiled
import asmble.run.jvm.ScriptContext
import cats.data.EitherT
import cats.effect.LiftIO
import cats.{Applicative, Functor, Monad}
import fluence.crypto.CryptoError
import fluence.vm.VmError.WasmVmError.{ApplyError, InvokeError}
import fluence.vm.VmError.{InitializationError, InternalVmError, NoSuchFnError, VmMemoryError}
import shapeless.Tuple

import scala.language.higherKinds
import scala.util.Try

/**
 * Wasm Module instance wrapper.
 *
 * @param name module name (can be empty)
 * @param instance wrapped instance of module
 * @param memory memory of this module
 */
class WasmModule(
  private val name: Option[String],
  private val memory: Option[WasmModuleMemory],
  private val instance: Any,
  private val allocateFunction: Option[WasmFunction],
  private val deallocateFunction: Option[WasmFunction],
  private val invokeFunction: Option[WasmFunction]
) {

  def getName: Option[String] = name

  /**
   * Allocates a memory region in Wasm module of supplied size by allocateFunction.
   *
   * @param size size of memory that need to be allocated
   */
  def allocate[F[_]: LiftIO: Monad](size: Int): EitherT[F, InvokeError, Int] =
    invokeWasmFunction(allocateFunction, size.asInstanceOf[AnyRef] :: Nil)
      // by specification, Wasm can return value only i32, i64, f32, f64 types
      .map(_.asInstanceOf[Number].intValue)

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

  def readMemory[F[_]: Monad](offset: Int, size: Int): EitherT[F, InvokeError, Array[Byte]] = memory match {
    case Some(wasmMemory) => wasmMemory.readBytes(offset, size)
    case _ =>
      EitherT.leftT(
        NoSuchFnError(s"Unable to find the function with name=readMemory in module with name=$this")
      )
  }

  def writeMemory[F[_]: Monad](offset: Int, injectedArray: Array[Byte]): EitherT[F, InvokeError, Unit] =
    memory match {
      case Some(wasmMemory) => wasmMemory.writeBytes(offset, injectedArray)
      case _ =>
        EitherT.leftT(
          NoSuchFnError(s"Unable to find the function with name=writeMemory in module with name=$this")
        )
    }

  /**
   * Returns hash of all significant inner state of this VM.
   *
   * @param hashFn a hash function
   */
  def computeHash[F[_]: Monad](
    hashFn: Array[Byte] ⇒ EitherT[F, CryptoError, Array[Byte]]
  ): EitherT[F, InternalVmError, Array[Byte]] =
    memory match {
      case Some(wasmMemory) ⇒
        for {
          memoryAsArray ← EitherT
            .fromEither[F](
              Try {
                // need a shallow ByteBuffer copy to avoid modifying the original one used by Asmble
                val wasmMemoryView = wasmMemory.memory.duplicate()
                wasmMemoryView.clear()
                val arr = new Array[Byte](wasmMemoryView.capacity())
                wasmMemoryView.get(arr)
                arr
              }.toEither
            )
            .leftMap { e ⇒
              InternalVmError(s"Copying memory to an array for module=$this failed", Some(e))
            }

          vmStateAsHash ← hashFn(memoryAsArray).leftMap { e ⇒
            InternalVmError(
              s"Getting internal state for module=$this failed",
              Some(e)
            )
          }

        } yield vmStateAsHash

      case None ⇒
        // Returning empty array is a temporary solution.
        // It's valid situation when a module doesn't have a memory.
        // When the Stack will be accessible we will return hash of the Stack with registers.
        EitherT.rightT(Array.emptyByteArray)
    }

  private def invokeWasmFunction[F[_]: LiftIO: Monad](
    wasmFunction: Option[WasmFunction],
    args: List[AnyRef]
  ): EitherT[F, InvokeError, AnyRef] = wasmFunction match {
    case Some(fn) => fn(instance, args)
    case _ =>
      EitherT.leftT(
        NoSuchFnError(s"Unable to find a function in module with name=$this")
      )
  }

  override def toString: String = name.getOrElse("<no-name>")
}

object WasmModule {

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
  ): Either[ApplyError, WasmModule] =
    for {

      moduleInstance <- Try(moduleDescription.instance(scriptContext)).toEither.left.map { e ⇒
        // TODO: method 'instance' must throw both an initialization error and a
        // Trap error, but now they can't be separated
        InitializationError(
          s"Unable to initialize module=${moduleDescription.getName}",
          Some(e)
        )
      }

      // getting memory field with reflection from module instance
      memory ← Try {
        // It's ok if a module doesn't have a memory
        val memoryMethod = Try(moduleInstance.getClass.getMethod("getMemory")).toOption
        memoryMethod.map(_.invoke(moduleInstance).asInstanceOf[ByteBuffer])
      }.toEither.left.map { e ⇒
        InitializationError(
          s"Unable to getting memory from module=${moduleDescription.getName}",
          Some(e)
        ): ApplyError
      }

      (allocMethod, deallocMethod, invokeMethod) = moduleDescription
        .getCls
        .getDeclaredMethods
        .toStream
        .filter(method => Modifier.isPublic(method.getModifiers))
        .map(method => WasmFunction(method.getName, method))
        .foldLeft((Option.empty[WasmFunction], Option.empty[WasmFunction], Option.empty[WasmFunction])) {
          case (acc @ (None, _, _), m @ WasmFunction(`allocationFunctionName`, _)) =>
            acc.copy(_1 = Some(m))

          case (acc @ (_, None, _), m @ WasmFunction(`deallocationFunctionName`, _)) =>
            acc.copy(_2 = Some(m))

          case (acc @ (_, _, None), m @ WasmFunction(`invokeFunctionName`, _)) =>
            acc.copy(_3 = Some(m))

          case (acc @ (_, _, _), _) => acc
        }

    } yield
      new WasmModule(
        Option(moduleDescription.getName),
        memory.map(memory => WasmModuleMemory(memory)),
        moduleInstance,
        allocMethod,
        deallocMethod,
        invokeMethod
      )

}
