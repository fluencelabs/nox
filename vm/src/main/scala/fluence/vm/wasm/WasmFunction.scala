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
import java.lang.reflect.Method

import cats.{Functor, Monad}
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import fluence.vm.VmError.TrapError
import fluence.vm.VmError.WasmVmError.InvokeError
import fluence.vm.wasm.module.ModuleInstance

import scala.language.higherKinds

trait WasmFunctionInvoker {

  def invokeWasmFunction[F[_]: LiftIO: Monad](
    moduleInstance: ModuleInstance,
    wasmFn: WasmFunction,
    args: List[AnyRef]
  ): EitherT[F, InvokeError, Int] =
    for {
      rawResult ← wasmFn(moduleInstance, args)

      // Despite our way of thinking about Wasm function return value type as one of (i32, i64, f32, f64) in
      // WasmModule context, there we can operate with Int (i32) values. It comes from our conventions about
      // Wasm modules design: they have to has only one export function as a user interface. It has to receive
      // and return a byte array, but since array can't be directly returns from Wasm part, It returns pointer
      // to in memory. And since Webassembly is only 32-bit now, Int(i32) is used as a pointer and return value
      // type. And after Wasm64 release, there should be additional logic to operate both with 32 and 64-bit
      // modules. TODO: fold with default value is just a temporary solution - after alloc/dealloc removal it
      // should be refactored to Either.fromOption
    } yield rawResult.fold(0)(_.intValue)

}

/**
 * Represent a Wasm function exported from a Wasm module.
 *
 * @param fnName a name of the function.
 * @param javaMethod a java method [[java.lang.reflect.Method]] used for calling the function.
 */
case class WasmFunction(
  fnName: String,
  javaMethod: Method
) {

  /**
   * Invokes the export from Wasm function with provided arguments.
   *
   * @param module a instance of Wasm Module compiled by Asmble,
   *              it is used as an object the underlying method is invoked from
   * @param args arguments for calling this function
   * @tparam F a monad with an ability to absorb 'IO'
   */
  def apply[F[_]: Functor: LiftIO](
    module: ModuleInstance,
    args: List[AnyRef]
  ): EitherT[F, InvokeError, Option[Number]] =
    EitherT(
      IO(javaMethod.invoke(module.moduleInstance, args: _*))
        .map(
          result ⇒
            // according to the current version of Wasm specification a Wasm method can return value
            // of only i32, i64, f32, f64 types or return nothing
            if (javaMethod.getReturnType == Void.TYPE) None else Option(result.asInstanceOf[Number])
        )
        .attempt
        .to[F]
    ).leftMap(e ⇒ TrapError(s"Function $this with args: $args was failed. Cause: ${e.getMessage}", Some(e)))

  override def toString: String = fnName
}
