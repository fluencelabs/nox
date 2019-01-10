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
import fluence.vm.AsmbleWasmVm._
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
class AsmbleWasmVm(
  private val modules: WasmModules,
  private val hasher: Hasher[Array[Byte], Array[Byte]],
) extends WasmVm {

  override def invoke[F[_]: LiftIO: Monad](
    moduleName: Option[String],
    fnArgument: Array[Byte]
  ): EitherT[F, InvokeError, Option[Array[Byte]]] = {
    val functionId = FunctionId(moduleName, AsmExtKt.getJavaIdent("invoke"))

    for {
      // Finds java method(Wasm function) in the index by function id
      wasmFn <- EitherT
        .fromOption(
          functionsIndex.get(functionId),
          NoSuchFnError(s"Unable to find a function with the name=$functionId")
        )

      preprocessedArguments <- preprocessFnArgument(fnArgument, wasmFn.module)

      // invoke the function
      invocationResult <- wasmFn[F](preprocessedArguments)

      // It is expected that callee (Wasm module) has to clean memory by itself because of otherwise
      // there can be some non-determinism (deterministic execution is very important for verification game
      // and this kind of non-determinism can break all verification game).
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

  /**
   * Preprocesses a Wasm function argument: injects it into Wasm module memory (through injectArrayIntoWasmModule)
   * and replaces with pointer to it in the Wasm module and size. This functions simply returns 0 :: 0 :: Nil
   * if supplied fnArgument was empty without any allocations in the Wasm side.
   *
   * @param fnArgument argument for calling this function
   * @param moduleInstance module instance used for injecting array to the Wasm memory
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def preprocessFnArgument[F[_]: LiftIO: Monad](
    fnArgument: Array[Byte],
    moduleInstance: ModuleInstance
  ): EitherT[F, InvokeError, List[AnyRef]] =
    if (fnArgument.isEmpty)
      EitherT.rightT[F, InvokeError](0.asInstanceOf[AnyRef] :: 0.asInstanceOf[AnyRef] :: Nil)
    else
      for {
        offset <- injectArrayIntoWasmModule(fnArgument, moduleInstance)
      } yield offset.asInstanceOf[AnyRef] :: fnArgument.length.asInstanceOf[AnyRef] :: Nil

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

          // need a shallow ByteBuffer copy to avoid modifying the original one used by Asmble
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

object AsmbleWasmVm {

  type WasmModules = List[ModuleInstance]
  type WasmFnIndex = Map[FunctionId, WasmFunction]

  /** Function id contains two components: optional module name and function name. */
  case class FunctionId(moduleName: Option[String], functionName: String) {
    override def toString =
      s"'${ModuleInstance.nameAsStr(moduleName)}.$functionName'"
  }

}
