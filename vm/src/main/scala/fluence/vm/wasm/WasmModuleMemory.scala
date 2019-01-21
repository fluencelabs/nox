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
import java.nio.{ByteBuffer, ByteOrder}

import fluence.vm.runThrowable
import cats.{Applicative, Monad}
import cats.data.EitherT
import fluence.crypto.CryptoError
import fluence.vm.VmError.{InternalVmError, VmMemoryError}
import fluence.vm.VmError.WasmVmError.GetVmStateError

import scala.language.higherKinds

final case class WasmModuleMemory(memory: ByteBuffer) {

  /**
   * Reads [offset, offset+size) region from the memory.
   *
   * @param offset offset from which read should be started
   * @param size bytes count to read
   */
  def readBytes[F[_]: Applicative](
    offset: Int,
    size: Int
  ): EitherT[F, VmMemoryError, Array[Byte]] =
    runThrowable(
      {
        // need a shallow ByteBuffer copy to avoid modifying the original one used by Asmble
        val wasmMemoryView = memory.duplicate()
        wasmMemoryView.order(ByteOrder.LITTLE_ENDIAN)

        val resultBuffer = new Array[Byte](size)
        // sets limit to capacity
        wasmMemoryView.clear()
        wasmMemoryView.position(offset)
        wasmMemoryView.get(resultBuffer)
        resultBuffer
      },
      e ⇒
        VmMemoryError(
          s"Reading from offset=$offset $size bytes failed",
          Some(e)
      )
    )

  /**
   * Writes array of bytes to memory.
   *
   * @param offset offset from which write should be started
   * @param injectedArray array that should be injected into the module memory
   */
  def writeBytes[F[_]: Applicative](
    offset: Int,
    injectedArray: Array[Byte]
  ): EitherT[F, VmMemoryError, Unit] =
    runThrowable(
      {
        // need a shallow ByteBuffer copy to avoid modifying the original one used by Asmble
        val wasmMemoryView = memory.duplicate()

        wasmMemoryView.position(offset)
        wasmMemoryView.put(injectedArray)
        ()
      },
      e ⇒ VmMemoryError(s"Writing to $offset failed", Some(e))
    )

  /**
   * Computes and returns hash of memory.
   *
   * @param hashFn a hash function
   */
  def computeMemoryHash[F[_]: Monad](
    hashFn: Array[Byte] ⇒ EitherT[F, CryptoError, Array[Byte]]
  ): EitherT[F, GetVmStateError, Array[Byte]] =
    for {
      memoryAsArray ← readBytes(0, memory.capacity())

      vmStateAsHash ← hashFn(memoryAsArray).leftMap { e ⇒
        InternalVmError(s"Computing wasm memory hash failed", Some(e)): GetVmStateError
      }
    } yield vmStateAsHash

}
