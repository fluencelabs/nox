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
import java.nio.ByteBuffer

import cats.Monad
import cats.data.EitherT
import fluence.crypto.CryptoError
import fluence.vm.VmError.InternalVmError

import scala.language.higherKinds
import scala.util.Try

case class WasmModuleState(private[vm] val memory: ByteBuffer) {

  /**
    * Returns hash of all significant inner state of this VM.
    *
    * @param hashFn a hash function
    */
  def computeStateHash[F[_]: Monad](
    hashFn: Array[Byte] ⇒ EitherT[F, CryptoError, Array[Byte]]):
  EitherT[F, InternalVmError, Array[Byte]] =
    memory match {
      case Some(mem) ⇒
        for {

          memoryAsArray ← EitherT
            .fromEither[F](
            Try {
              // need a shallow ByteBuffer copy to avoid modifying the original one used by Asmble
              val wasmMemoryView = mem.duplicate()
              wasmMemoryView.clear()
              val arr = new Array[Byte](wasmMemoryView.capacity())
              wasmMemoryView.get(arr)
              arr
            }.toEither
          )
            .leftMap { e ⇒
              InternalVmError(
                s"Presenting memory as an array for module=${nameAsStr(name)} failed",
                Some(e)
              )
            }

          vmStateAsHash ← hashFn(memoryAsArray).leftMap { e ⇒
            InternalVmError(
              s"Getting internal state for module=${nameAsStr(name)} failed",
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
}
