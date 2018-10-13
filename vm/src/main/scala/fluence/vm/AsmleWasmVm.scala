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
import java.lang.reflect.Method
import java.nio.ByteOrder

import asmble.compile.jvm.AsmExtKt
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import cats.{Functor, Monad}
import fluence.crypto.Crypto.Hasher
import fluence.vm.VmError.WasmVmError.{GetVmStateError, InvokeError}
import fluence.vm.VmError.{NoSuchFnError, _}
import fluence.vm.AsmleWasmVm._
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.Try

/**
 * Base implementation of [[WasmVm]].
 *
 * '''Note!!! This implementation isn't thread-safe. The provision of calls
 * linearization is the task of the caller side.'''
 *
 * @param functionsIndex the index for fast function searching. Contains all the
 *                     information needed to execute any function.
 * @param modules list of Wasm modules
 * @param hasher a hash function provider
 * @param allocateFunctionName name of function that will be used for allocation
 *                             memory in the Wasm part
 * @param deallocateFunctionName name of a function that will be used for freeing memory
 *                               that was previously allocated by allocateFunction
 */
class AsmleWasmVm(
  private val functionsIndex: WasmFnIndex,
  private val modules: WasmModules,
  private val hasher: Hasher[Array[Byte], Array[Byte]],
  private val allocateFunctionName: String,
  private val deallocateFunctionName: String
) extends WasmVm {

  // TODO: now it is assumed that allocation/deallocation functions placed together in the first module.
  // In the future it has to be refactored.
  // TODO: add handling of empty modules list.
  private val allocateFunction: Option[WasmFunction] =
    functionsIndex.get(FunctionId(modules.head.name, AsmExtKt.getJavaIdent(allocateFunctionName)))

  private val deallocateFunction: Option[WasmFunction] =
    functionsIndex.get(FunctionId(modules.head.name, AsmExtKt.getJavaIdent(deallocateFunctionName)))

  override def invoke[F[_]: LiftIO: Monad](
    moduleName: Option[String],
    fnName: String,
    fnArgument: Option[Array[Byte]]
  ): EitherT[F, InvokeError, Option[Array[Byte]]] = {
    val functionId = FunctionId(moduleName, AsmExtKt.getJavaIdent(fnName))

    for {
      // Finds java method(Wasm function) in the index by function id
      wasmFn <- EitherT
        .fromOption(
          functionsIndex.get(functionId),
          NoSuchFnError(s"Unable to find a function with the name=$functionId")
        )

      // invoke the function
      invocationResult <- fnArgument match {
        case Some(arg) => for {
          offset <- injectArrayIntoWasmModule(arg, wasmFn.module)
        } yield wasmFn[F](offset.asInstanceOf[AnyRef] :: arg.length.asInstanceOf[AnyRef] :: Nil)
        case None => wasmFn[F](Nil)
      }

      extractedResult <- if (wasmFn.javaMethod.getReturnType == Void.TYPE) {
        EitherT.rightT[F, InvokeError](None)
      } else {
        for {
          offset <- EitherT.fromEither(Try(invocationResult.toString.toInt).toEither).leftMap { e ⇒
            VmMemoryError(s"Trying to extract result from incorrect offset=$invocationResult", Some(e))
          }
          extractedResult <- extractResultFromWasmModule(offset, wasmFn.module).map(Option(_))
        } yield extractedResult
      }
    } yield extractedResult

  }

  override def getVmState[F[_]: LiftIO: Monad]: EitherT[F, GetVmStateError, ByteVector] =
    modules
      .foldLeft(EitherT.rightT[F, GetVmStateError](Array[Byte]())) {
        case (acc, instance) ⇒
          for {
            moduleStateHash ← instance
              .innerState(arr ⇒ hasher[F](arr))

            prevModulesHash ← acc

            concatHashes = Array.concat(moduleStateHash, prevModulesHash)

            resultHash ← hasher(concatHashes)
              .leftMap(e ⇒ InternalVmError(s"Getting VM state for module=$instance failed", Some(e)): GetVmStateError)

          } yield resultHash
      }
      .map(ByteVector(_))

  // TODO : In the future, it should be rewritten with cats.effect.resource
  /**
   * Allocates memory in Wasm module of supplied size by allocateFunction.
   *
   * @param size size of memory that need to be allocated
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def allocate[F[_]: LiftIO: Monad](size: Int): EitherT[F, InvokeError, AnyRef] = {
    allocateFunction match {
      case Some(fn) => fn(size.asInstanceOf[AnyRef] :: Nil)
      case _ =>
        EitherT.leftT(
          NoSuchFnError(s"Unable to find the function for memory allocation with the name=$allocateFunctionName")
        )
    }
  }

  /**
   * Deallocates previously allocated memory in Wasm module by deallocateFunction.
   *
   * @param offset address of memory to deallocate
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def deallocate[F[_]: LiftIO: Monad](offset: Int, size: Int): EitherT[F, InvokeError, AnyRef] = {
    deallocateFunction match {
      case Some(fn) => fn(offset.asInstanceOf[AnyRef] :: size.asInstanceOf[AnyRef] :: Nil)
      case _ =>
       EitherT.leftT(
          NoSuchFnError(s"Unable to find the function for memory deallocation with the name=$deallocateFunctionName")
        )
    }
  }

  /**
   * Injects given string into Wasm module memory.
   *
   * @param injectedArray array that should be inserted into Wasm module memory
   * @param moduleInstance module instance used as a provider for Wasm module memory access
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def injectArrayIntoWasmModule[F[_]: LiftIO: Monad](
    injectedArray: Array[Byte],
    moduleInstance: ModuleInstance
  ): EitherT[F, InvokeError, Int] =
    for {
      // In the current version, it is possible for Wasm module to have allocation/deallocation
      // functions but doesn't have memory (ByteBuffer instance). Also since it is used getMemory
      // Wasm function for getting ByteBuffer instance, it is also possible to don't have memory
      // due to possible absence of this function in the Wasm module.
      wasmMemory <- EitherT.fromOption(
        moduleInstance.memory,
        VmMemoryError(s"Trying to use absent Wasm memory while injecting array=$injectedArray")
      )

      offset <- allocate(injectedArray.length)

      resultOffset <- EitherT
        .fromEither(Try {
          val convertedOffset = offset.toString.toInt
          val wasmMemoryView = wasmMemory.duplicate()

          wasmMemoryView.position(convertedOffset)
          wasmMemoryView.put(injectedArray)

          convertedOffset
        }.toEither)
        .leftMap { e ⇒
          VmMemoryError(s"The Wasm allocation function returned incorrect offset=$offset", Some(e))
        }: EitherT[F, InvokeError, Int]

    } yield resultOffset

  /**
   * Extracts (reads and deletes) result from the given offset from Wasm module memory.
   *
   * @param offset offset into Wasm module memory where a string is located
   * @param moduleInstance module instance used as a provider for Wasm module memory access
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def extractResultFromWasmModule[F[_]: LiftIO: Monad](
    offset: Int,
    moduleInstance: ModuleInstance
  ): EitherT[F, InvokeError, Array[Byte]] =
    for {
      extractedResult <- readResultFromWasmModule(offset, moduleInstance)
      // TODO : string deallocation from scala-part should be additionally investigated - it seems
      // that this version of deletion doesn't compatible with current idea of verification game
      intBytesSize = 4
      _ <- deallocate(offset, extractedResult.length + intBytesSize)
    } yield extractedResult

  /**
   * Reads result from the given offset from Wasm module memory.
   *
   * @param offset offset into Wasm module memory where a string is located
   * @param moduleInstance module instance used as a provider for Wasm module memory access
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def readResultFromWasmModule[F[_]: LiftIO: Monad](
    offset: Int,
    moduleInstance: ModuleInstance
  ): EitherT[F, InvokeError, Array[Byte]] =
    for {
      wasmMemory <- EitherT.fromOption(
        moduleInstance.memory,
        VmMemoryError(s"Trying to use absent Wasm memory while reading string from the offset=$offset")
      )

      readResult <- EitherT
        .fromEither(
          Try {
            val wasmMemoryView = wasmMemory.duplicate()
            wasmMemoryView.order(ByteOrder.LITTLE_ENDIAN)

            // each result has the next structure in Wasm memory: | size (4 bytes) | result buffer (size bytes) |
            val resultSize = wasmMemoryView.getInt(offset)
            // size of Int in bytes
            val intBytesSize = 4

            val resultBuffer = new Array[Byte](resultSize)
            wasmMemoryView.position(offset + intBytesSize)
            wasmMemoryView.get(resultBuffer)
            resultBuffer
          }.toEither
        )
        .leftMap { e =>
          VmMemoryError(
            s"String reading from offset=$offset failed",
            Some(e)
          )
        }: EitherT[F, InvokeError, Array[Byte]]

    } yield readResult

}

object AsmleWasmVm {

  type WasmModules = List[ModuleInstance]
  type WasmFnIndex = Map[FunctionId, WasmFunction]

  /** Function id contains two components: optional module name and function name. */
  case class FunctionId(moduleName: Option[String], functionName: String) {
    override def toString =
      s"'${ModuleInstance.nameAsStr(moduleName)}.$functionName'"
  }

  /**
   * Representation for each Wasm function. Contains reference to module instance
   * and java method [[java.lang.reflect.Method]].
   *
   * @param javaMethod a java method [[java.lang.reflect.Method]] for calling function.
   * @param module the object the underlying method is invoked from.
   *               This is an instance for the current module, it contains
   *               all inner state of the module, like memory.
   */
  case class WasmFunction(
    javaMethod: Method,
    module: ModuleInstance
  ) {

    /**
     * Invokes this function with arguments.
     *
     * @param args arguments for calling this function.
     * @tparam F a monad with an ability to absorb 'IO'
     */
    def apply[F[_]: Functor: LiftIO](args: List[AnyRef]): EitherT[F, InvokeError, AnyRef] =
      EitherT(IO(javaMethod.invoke(module.instance, args: _*)).attempt.to[F])
        .leftMap(e ⇒ TrapError(s"Function $this with args: $args was failed", Some(e)))

    override def toString: String = "test"
  }

}
