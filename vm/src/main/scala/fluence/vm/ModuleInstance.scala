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
import java.nio.ByteBuffer

import asmble.run.jvm.Module.Compiled
import asmble.run.jvm.ScriptContext
import cats.Monad
import cats.data.EitherT
import fluence.crypto.CryptoError
import fluence.vm.ModuleInstance.nameAsStr
import fluence.vm.VmError.WasmVmError.ApplyError
import fluence.vm.VmError.{InitializationError, InternalVmError}

import scala.language.higherKinds
import scala.util.Try

/**
 * WASM Module instance wrapper.
 *
 * @param name optional module name
 * @param instance wrapped instance of module
 * @param memory memory of this module
 */
case class ModuleInstance(
  name: Option[String],
  instance: Any,
  private[vm] val memory: Option[ByteBuffer]
) {

  /**
   * Returns hash of all significant inner state of this VM.
   *
   * @param hashFn a hash function
   */
  def innerState[F[_]: Monad](
    hashFn: Array[Byte] ⇒ EitherT[F, CryptoError, Array[Byte]]
  ): EitherT[F, InternalVmError, Array[Byte]] =
    memory match {
      case Some(mem) ⇒
        for {

          memoryAsArray ← EitherT
            .fromEither[F](
              Try {
                val arr = new Array[Byte](mem.remaining())
                // Duplicate is required for reaching idempotent reading ByteBuffer(BB).
                // ''ByteBuffer.get'' change inner BB state, for preventing this
                // we create a thin copy of this BB. The new buffer's capacity,
                // limit, position, and mark values will be identical to those of
                // original buffer, but the content(bytes) will be shared (bytes won't be copied).
                // After reading all bytes, duplicate will be collected by GC
                mem.duplicate().get(arr, 0, arr.length)
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
        // Returning empty array is temporal solution.
        // It's valid situation when a module doesn't have a memory.
        // When the Stack will be accessible we will return hash of the Stack with registers.
        EitherT.rightT(Array.emptyByteArray)
    }

  override def toString: String = s"Module(${nameAsStr(name)}, memory=$memory)"
}

object ModuleInstance {

  /**
   * Creates instance for specified module.
   *
   * @param moduleDescription description of the module
   * @param scriptContext context for the module operation
   */
  def apply(
    moduleDescription: Compiled,
    scriptContext: ScriptContext
  ): Either[ApplyError, ModuleInstance] =
    for {

      // creating module instance
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

    } yield ModuleInstance(Option(moduleDescription.getName), moduleInstance, memory)

  def nameAsStr(moduleName: Option[String]): String = moduleName.getOrElse("<no-name>")

  private def nameAsStr(moduleName: String): String = nameAsStr(Option(moduleName))

}
