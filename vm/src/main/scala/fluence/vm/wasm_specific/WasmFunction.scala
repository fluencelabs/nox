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
import java.lang.reflect.Method

import cats.Functor
import cats.data.EitherT
import cats.effect.{IO, LiftIO}
import fluence.vm.VmError.TrapError
import fluence.vm.VmError.WasmVmError.InvokeError

import scala.language.higherKinds

/**
  * Representation for each Wasm function. Contains reference to module instance
  * and java method [[java.lang.reflect.Method]].
  *
  * @param fnName a name of function.
  * @param javaMethod a java method [[java.lang.reflect.Method]] for calling function.
  */
case class WasmFunction(
  private val fnName: String,
  private val javaMethod: Method,
) {

  /**
    * Invokes this function with arguments.
    *
    * @param module the object the underlying method is invoked from.
    *               This is an instance for the current module, it contains
    *               all inner state of the module, like memory.
    * @param args arguments for calling this function.
    * @tparam F a monad with an ability to absorb 'IO'
    */
  def apply[F[_]: Functor: LiftIO](module: Any, args: List[AnyRef]): EitherT[F, InvokeError, AnyRef] =
    EitherT(
      IO(
        javaMethod.invoke(module, args: _*))
        .attempt
        .to[F]
    ).leftMap(e ⇒
      TrapError(s"Function $this with args: $args was failed", Some(e))
    )

  override def toString: String = fnId.toString
}
