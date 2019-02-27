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

import cats.Functor
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import fluence.vm.VmError.TrapError
import fluence.vm.VmError.WasmVmError.InvokeError

import scala.language.higherKinds

/**
 * Represent a Wasm function exported from a Wasm module.
 *
 * @param fnName a name of the function.
 * @param javaMethod a java method [[java.lang.reflect.Method]] used for calling the function.
 */
case class WasmFunction(
  fnName: String,
  javaMethod: Method,
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
    module: Any,
    args: List[AnyRef]
  ): EitherT[F, InvokeError, Option[Number]] =
    EitherT(
      IO(javaMethod.invoke(module, args: _*))
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
