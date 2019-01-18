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

import java.nio.{ByteBuffer, ByteOrder}

import cats.data.EitherT
import cats.effect.LiftIO
import cats.Monad
import fluence.crypto.Crypto.Hasher
import fluence.vm.VmError.WasmVmError.{GetVmStateError, InvokeError}
import fluence.vm.VmError.{NoSuchModuleError, _}
import fluence.vm.wasm.WasmModule
import scodec.bits.ByteVector
import WasmVm._

import scala.language.higherKinds

/**
 * Base implementation of [[WasmVm]].
 *
 * '''Note!!! This implementation isn't thread-safe. The provision of calls
 * linearization is the task of the caller side.'''
 *
 * @param modules list of Wasm modules
 * @param hasher a hash function provider
 */
class AsmbleWasmVm(
  private val modules: ModuleIndex,
  private val hasher: Hasher[Array[Byte], Array[Byte]]
) extends WasmVm {

  // size in bytes of pointer type in Wasm VM (can be different after Wasm64 release)
  val WasmPointerSize = 4

  override def invoke[F[_]: LiftIO: Monad](
    moduleName: Option[String],
    fnArgument: Array[Byte]
  ): EitherT[F, InvokeError, Array[Byte]] =
    for {
      wasmModule <- EitherT
        .fromOption(
          modules.get(moduleName),
          NoSuchModuleError(s"Unable to find a module with the name=$moduleName")
        )
      preprocessedArgument <- preprocessFnArgument(fnArgument, wasmModule)
      invocationResult <- wasmModule.invoke(preprocessedArgument)

      // It is expected that callee (Wasm module) has to clean memory by itself because of otherwise
      // there can be some non-determinism (deterministic execution is very important for verification game
      // and this kind of non-determinism can break all verification game).
      offset <- runThrowable(
        invocationResult.toString.toInt,
        e ⇒ VmMemoryError(s"Trying to extract result from incorrect offset=$invocationResult", Some(e))
      )
      extractedResult <- extractResultFromWasmModule(offset, wasmModule)

    } yield extractedResult

  override def getVmState[F[_]: LiftIO: Monad]: EitherT[F, GetVmStateError, ByteVector] =
    modules
      .foldLeft(EitherT.rightT[F, GetVmStateError](Array[Byte]())) {
        case (acc, (moduleName, module)) ⇒
          for {
            moduleStateHash ← module
              .computeStateHash(arr ⇒ hasher[F](arr))

            prevModulesHash ← acc

            concatHashes = Array.concat(moduleStateHash, prevModulesHash)

            // TODO : There is known the 2nd preimage attack to such scheme with the same hash function
            //  for leaves and nodes.
            resultHash ← hasher(concatHashes).leftMap { e ⇒
              InternalVmError(s"Getting VM state for module=$moduleName failed", Some(e)): GetVmStateError
            }

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
    moduleInstance: WasmModule
  ): EitherT[F, InvokeError, List[AnyRef]] =
    if (fnArgument.isEmpty)
      EitherT.rightT[F, InvokeError](0.asInstanceOf[AnyRef] :: 0.asInstanceOf[AnyRef] :: Nil)
    else
      for {
        offset <- moduleInstance.allocate(fnArgument.length)
        _ <- moduleInstance.writeMemory(offset, fnArgument).leftMap(e => e: InvokeError)

      } yield offset.asInstanceOf[AnyRef] :: fnArgument.length.asInstanceOf[AnyRef] :: Nil

  /**
   * Extracts (reads and deletes) result from the given offset from Wasm module memory.
   *
   * @param offset offset into Wasm module memory where a string is located
   * @param moduleInstance module instance used as a provider for Wasm module memory access
   * @tparam F a monad with an ability to absorb 'IO'
   */
  private def extractResultFromWasmModule[F[_]: LiftIO: Monad](
    offset: Int,
    moduleInstance: WasmModule
  ): EitherT[F, InvokeError, Array[Byte]] =
    for {
      // each result has the next structure in Wasm memory: | size (wasmPointerSize bytes) | result buffer (size bytes) |
      resultSize <- moduleInstance.readMemory(offset, WasmPointerSize)

      // convert ArrayByte to Int
      resultSize <- runThrowable(
        ByteBuffer.wrap(resultSize).order(ByteOrder.LITTLE_ENDIAN).getInt(),
        e ⇒ VmMemoryError(s"Trying to extract result from incorrect offset=$resultSize", Some(e))
      )

      extractedResult <- moduleInstance.readMemory(offset + WasmPointerSize, resultSize)

      // TODO : string deallocation from scala-part should be additionally investigated - it seems
      // that this version of deletion doesn't compatible with current idea of verification game
      _ <- moduleInstance.deallocate(offset, extractedResult.length + WasmPointerSize)
    } yield extractedResult

}
