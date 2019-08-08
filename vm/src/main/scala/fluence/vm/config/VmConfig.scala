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

package fluence.vm.config

import cats.Monad
import cats.data.EitherT
import com.typesafe.config.Config
import fluence.vm.VmError.InternalVmError
import fluence.vm.VmError.WasmVmError.ApplyError
import net.ceedubs.ficus.readers.namemappers.implicits.hyphenCase
import net.ceedubs.ficus.readers.ArbitraryTypeReader._
import net.ceedubs.ficus.readers.EnumerationReader._
import net.ceedubs.ficus.Ficus._
import net.ceedubs.ficus.readers.ArbitraryTypeReader._

import scala.language.higherKinds
import scala.util.Try

/**
 * WasmVm settings.
 *
 * @param defaultMaxMemPages the maximum count of memory pages when a module doesn't say
 * @param specTestRegister if true, registers the spec test harness as 'spectest'
 * @param loggerRegister if > 0, registers the logger Wasm module as 'logger'
 *                       with specified count of memory pages
 * @param allocateFunctionName name of a function that should be called for allocation memory
 *                             (used for passing complex data structures)
 * @param deallocateFunctionName name of a function that should be called for deallocation
 *                               of previously allocated memory
 * @param invokeFunctionName name of main module handler function
 */
case class VmConfig(
  defaultMaxMemPages: Int,
  specTestRegister: Boolean,
  loggerRegister: Int,
  chunkSize: Int,
  allocateFunctionName: String,
  deallocateFunctionName: String,
  invokeFunctionName: String
)

object VmConfig {

  def readT[F[_]: Monad](namespace: String, conf: ⇒ Config): EitherT[F, ApplyError, VmConfig] =
    EitherT
      .fromEither[F](
        Try(
          conf.getConfig(namespace).as[VmConfig]
        ).toEither
      )
      .leftMap(
        e ⇒
          InternalVmError(
            s"Unable to read a config for the namespace=$namespace",
            Some(e)
        )
      )
}
