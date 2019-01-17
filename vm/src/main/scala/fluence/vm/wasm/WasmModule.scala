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
import fluence.vm.runThrowable

import scala.language.higherKinds
import scala.util.Try

/**
 * Wasm Module instance wrapper.
 *
 * @param name optional module name (can be empty)
 * @param wasmMemory memory of this module
 * @param instance wrapped instance of module
 */
class WasmModule(
  private val name: Option[String],
  private val wasmMemory: Option[WasmModuleMemory],
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

  /**
   * Deallocates a previously allocated memory region in Wasm module by deallocateFunction.
   * This function is a temporary solution and should be removed in the nearest future (that why
   * it returns Int not Unit).
   *
   * @param offset address of the memory region to deallocate
   * @param size size of memory region to deallocate
   */
  def deallocate[F[_]: LiftIO: Monad](offset: Int, size: Int): EitherT[F, InvokeError, Int] =
    invokeWasmFunction(deallocateFunction, offset.asInstanceOf[AnyRef] :: size.asInstanceOf[AnyRef] :: Nil)

  /**
   * Invokes invokeFunction which exported from Wasm module with provided arguments.
   *
   * @param args arguments for invokeFunction
   */
  def invoke[F[_]: LiftIO: Monad](args: List[AnyRef]): EitherT[F, InvokeError, Int] =
    invokeWasmFunction(invokeFunction, args)

  /**
   * Invokes invokeFunction which exported from Wasm module with provided arguments.
   *
   * @param args arguments for invokeFunction
   */
  def readMemory[F[_]: Monad](offset: Int, size: Int): EitherT[F, InvokeError, Array[Byte]] =
    wasmMemory.fold(
      EitherT.leftT[F, Array[Byte]](
        VmMemoryError(s"Unable to find the function with name=readMemory in module with name=$this"): InvokeError
      )
    )(_.readBytes(offset, size))

  def writeMemory[F[_]: Monad](offset: Int, injectedArray: Array[Byte]): EitherT[F, InvokeError, Unit] =
    wasmMemory.fold(
      EitherT.leftT[F, Unit](
        VmMemoryError(s"Unable to find the function with name=readMemory in module with name=$this"): InvokeError
      )
    )(_.writeBytes(offset, injectedArray))

  /**
   * Returns hash of all significant inner state of this VM.
   *
   * @param hashFn a hash function
   */
  def computeStateHash[F[_]: Monad](
    hashFn: Array[Byte] ⇒ EitherT[F, CryptoError, Array[Byte]]
  ): EitherT[F, InternalVmError, Array[Byte]] =
    wasmMemory.fold(
      EitherT.rightT[F, InternalVmError](
        Array.emptyByteArray
      )
    )(
      wasmMemory =>
        for {
          memoryAsArray ← runThrowable(
            {
              // need a shallow ByteBuffer copy to avoid modifying the original one used by Asmble
              val wasmMemoryView = wasmMemory.memory.duplicate()
              wasmMemoryView.clear()
              val arr = new Array[Byte](wasmMemoryView.capacity())
              wasmMemoryView.get(arr)
              arr
            },
            e ⇒ InternalVmError(s"Copying memory to an array for module=$this failed", Some(e))
          )

          vmStateAsHash ← hashFn(memoryAsArray).leftMap { e ⇒
            InternalVmError(s"Getting internal state for module=$this failed", Some(e))
          }
        } yield vmStateAsHash
    )

  private def invokeWasmFunction[F[_]: LiftIO: Monad](
    wasmFunction: Option[WasmFunction],
    args: List[AnyRef]
  ): EitherT[F, InvokeError, Int] =
    wasmFunction.fold(
      EitherT.leftT[F, Int](
        NoSuchFnError(s"Unable to find a function in module with name=$this"): InvokeError
      )
    )(
      fn =>
        for {
          rawResult <- fn(instance, args)

          // Despite our way of thinking about wasm function return value type as one of (i32, i64, f32, f64) in
          // WasmModule context, there we can operate with Int (i32) values. It comes from our conventions about
          // Wasm modules design: they have to has only one export function as a user interface and one for memory
          // allocation. The first one receives and returns byte array by a pointer and the second one used for
          // allocating memory for array passing. Since Webassembly is only 32-bit now, Int(i32) is used as a
          // return value type. And after Wasm64 release, there should be additional logic to operate both with
          // 32 and 64-bit modules (TODO).
          result <- EitherT.fromOption(
            rawResult,
            InternalVmError(s"Returned value of function=${fn.fnName} from module=$this is None"): InvokeError
          )
        } yield result.intValue
    )

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
        // TODO: method 'instance' can throw both an initialization error and a
        // Trap error, but now they can't be separated
        InitializationError(
          s"Unable to initialize module=${moduleDescription.getName}",
          Some(e)
        )
      }

      // TODO: there are two ways of getting memory from Wasm module: by exported getMemory function or
      // through moduleInstance.getMem interface. Based on Asmble source code It seems that the behavior
      // of getMemory is returning smth like 'export memory' in terms of Wasm specification. getMemory
      // is more suitable for the scenario when at first Wasm module translates to Java class and then
      // compiled with the rest of Java code. In our approach, it needs to use reflection for invocation.
      // Another issue lies in the fact that Asmble may not generate this function. But ByteBuffer presence
      // in every translated module (it is created in ctors of generated Java class). The second approach
      // of accessing is more complicated because Mem provides a more low level and nonsuitable interface
      // for use, but it gives us the capability for getting memory for any module. And in the future, we
      // should move to it.
      memory ← Try {
        val getMemoryMethod = Try(
          moduleInstance.getClass.getMethod("getMemory")
        ).toOption
        getMemoryMethod.map(_.invoke(moduleInstance).asInstanceOf[ByteBuffer])
      }.toEither.left.map { e ⇒
        InitializationError(
          s"Unable to getting memory from module=${moduleDescription.getName}",
          Some(e)
        ): ApplyError
      }

      (allocMethod, deallocMethod, invokeMethod) = moduleDescription.getCls.getDeclaredMethods.toStream
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
