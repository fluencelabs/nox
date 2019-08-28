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
import fluence.vm.utils.safelyRunThrowable
import net.ceedubs.ficus.Ficus._
import net.ceedubs.ficus.readers.ArbitraryTypeReader._
import scala.language.higherKinds

/**
 * Main module settings.
 *
 * @param name a name of the main module (None means absence of name section in a Wasm module)
 * @param allocateFunctionName name of a function that should be called for allocation memory
 * (used for passing complex data structures)
 * @param deallocateFunctionName name of a function that should be called for deallocation
 * of previously allocated memory
 * @param invokeFunctionName name of main module handler function
 */
case class MainModuleConfig(
  name: Option[String],
  allocateFunctionName: String,
  deallocateFunctionName: String,
  invokeFunctionName: String
)

/**
 * Environment module settings.
 *
 * @param name a name of the environment module
 * @param spentGasFunctionName a name of the function that returns spent gas
 * @param clearStateFunction a name of the function that clears a state of the environment module
 */
case class EnvModuleConfig(
  name: String,
  spentGasFunctionName: String,
  clearStateFunction: String
)

/**
 * WasmVm settings.
 *
 * @param defaultMaxMemPages the maximum count of memory pages when a module doesn't say
 * @param specTestEnabled if true, registers the spec test harness as 'spectest'
 * @param loggerModuleEnabled if set, registers the logger Wasm module as 'logger'
 * @param chunkSize a size of the memory chunks, that memory will be split into
 * @param mainModuleConfig settings for the main module
 * @param envModuleConfig settings for the environment module
 */
case class VmConfig(
  defaultMaxMemPages: Int,
  specTestEnabled: Boolean,
  loggerModuleEnabled: Boolean,
  chunkSize: Int,
  mainModuleConfig: MainModuleConfig,
  envModuleConfig: EnvModuleConfig
)

object VmConfig {

  def readT[F[_]: Monad](namespace: String, conf: ⇒ Config): EitherT[F, ApplyError, VmConfig] =
    safelyRunThrowable(
      conf.getConfig(namespace).as[VmConfig],
      e ⇒
        InternalVmError(
          s"Unable to read a config for the namespace=$namespace",
          Some(e)
        )
    )
}
