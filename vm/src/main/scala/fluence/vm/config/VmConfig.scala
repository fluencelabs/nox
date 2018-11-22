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

import pureconfig.{CamelCase, ConfigFieldMapping, ProductHint}
import pureconfig.error.ConfigReaderFailures

import scala.util.control.NoStackTrace

/**
 * WasmVm settings.
 *
 * @param defaultMaxMemPages the maximum count of memory pages when a module doesn't say
 * @param specTestRegister if true, registers the spec test harness as 'spectest'.
 * @param loggerRegister if > 0, registers the logger Wasm module as 'logger'
 *                        with specified number of memory pages.
 * @param allocateFunctionName name of function that should be called for allocation memory
 *                             for passing complex data structure
 * @param deallocateFunctionName name of function that should be called for deallocation
 *                              of previously allocated memory
 */
case class VmConfig(
  defaultMaxMemPages: Int,
  specTestRegister: Boolean,
  loggerRegister: Int,
  allocateFunctionName: String,
  deallocateFunctionName: String
)

object VmConfig {

  implicit def hint[T]: ProductHint[T] = ProductHint[T](ConfigFieldMapping(CamelCase, CamelCase))

  case class ConfigError(failures: ConfigReaderFailures) extends NoStackTrace

}
