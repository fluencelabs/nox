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
import fluence.vm.VmError.{InternalVmError, VmMemoryError}
import fluence.vm.utils.safelyRunThrowable
import scodec.bits.ByteVector
import fluence.vm.wasm.module.{EnvModule, MainWasmModule, WasmModule}

import scala.language.higherKinds

/**
 * Base implementation of [[WasmVm]].
 *
 * '''Note!!! This implementation isn't thread-safe. The provision of calls
 * linearization is the task of the caller side.'''
 *
 * @param mainModule a main Wasm module which used for function invoking
 * @param envModule a environment module which used for some auxiliary functions such as gas metering
 * @param sideModules a list of side modules used in VM state computing
 * @param hasher a hash function provider used for VM state hash computing
 */
class AsmbleWasmVm(
  private val mainModule: MainWasmModule,
  private val envModule: EnvModule,
  private val sideModules: Seq[WasmModule],
  private val hasher: Hasher[Array[Byte], Array[Byte]]
) extends WasmVm {

  // size in bytes of pointer type in Wasm VM (can be different after Wasm64 release)
  private val WasmPointerSize = 4

  override def invoke[F[_]: LiftIO: Monad](
    fnArgument: Array[Byte]
  ): EitherT[F, InvokeError, InvocationResult] =
    for {
      preprocessedArgument ← loadArgToMemory(fnArgument)
      resultOffset ← mainModule.invoke(preprocessedArgument)
      _ <- envModule.clearSpentGas()

      // It is expected that callee (Wasm module) has to clean memory by itself because of otherwise
      // there can be some non-determinism (deterministic execution is very important for verification game
      // and this kind of non-determinism can break all verification game).
      extractedResult ← extractResultFromWasmModule(resultOffset)

      spentGas <- envModule.getSpentGas()

    } yield InvocationResult(extractedResult, spentGas)

  override def getVmState[F[_]: LiftIO: Monad]: EitherT[F, GetVmStateError, ByteVector] =
    for {
      mainModuleHash <- mainModule.computeStateHash()

      sideModulesHash <- sideModules
        .foldLeft(EitherT.rightT[F, GetVmStateError](Array[Byte]())) {
          case (acc, module) ⇒
            for {
              moduleStateHash ← module.computeStateHash()

              prevModulesHash ← acc

              concatHashes = Array.concat(moduleStateHash, prevModulesHash)

              // TODO : It is known the 2nd preimage attack to such scheme with the same hash function
              // for leaves and nodes.
              resultHash ← hasher(concatHashes).leftMap { e ⇒
                InternalVmError(s"Getting VM state for module=${module.name} failed", Some(e)): GetVmStateError
              }

            } yield resultHash
        }

      resultHash <- hasher(Array.concat(sideModulesHash, mainModuleHash)).map(ByteVector(_)).leftMap { e ⇒
        InternalVmError(s"Getting VM state for modules failed", Some(e)): GetVmStateError
      }

    } yield resultHash

  /**
   * Preprocesses Wasm function argument array by injecting it into Wasm module memory and replacing by pointer to
   * it in the Wasm module and its size. This functions simply returns 0 :: 0 :: Nil if supplied fnArgument was empty
   * without any allocations on the Wasm side.
   *
   * @param fnArgument an argument that should be preprocessed
   */
  private def loadArgToMemory[F[_]: LiftIO: Monad](
    fnArgument: Array[Byte]
  ): EitherT[F, InvokeError, List[AnyRef]] =
    if (fnArgument.isEmpty)
      EitherT.rightT[F, InvokeError](
        Int.box(0) :: Int.box(0) :: Nil
      )
    else
      for {
        offset ← mainModule.allocate(fnArgument.length)
        _ ← mainModule.writeMemory(offset, fnArgument).leftMap(e ⇒ e: InvokeError)

      } yield Int.box(offset) :: Int.box(fnArgument.length) :: Nil

  /**
   * Extracts (reads and deletes) result from the given offset from Wasm module memory.
   *
   * @param offset offset into Wasm module memory where a result is located
   */
  private def extractResultFromWasmModule[F[_]: LiftIO: Monad](
    offset: Int
  ): EitherT[F, InvokeError, Array[Byte]] =
    for {
      // each result has the next structure in Wasm memory: | size (wasmPointerSize bytes) | result buffer (size bytes) |
      rawResultSize ← mainModule.readMemory(offset, WasmPointerSize)

      // convert ArrayByte to Int
      resultSize ← safelyRunThrowable(
        ByteBuffer.wrap(rawResultSize).order(ByteOrder.LITTLE_ENDIAN).getInt(),
        e ⇒ VmMemoryError(s"Trying to extract result from incorrect offset=$rawResultSize", Some(e))
      )

      extractedResult ← mainModule.readMemory(offset + WasmPointerSize, resultSize)

      // TODO : string deallocation from scala-part should be additionally investigated - it seems
      // that this version of deletion doesn't compatible with current idea of verification game
      _ ← mainModule.deallocate(offset, extractedResult.length + WasmPointerSize)

    } yield extractedResult

}
