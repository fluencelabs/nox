/*
 * Copyright (C) 2018  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.vm.config

import pureconfig.{CamelCase, ConfigFieldMapping, ProductHint}
import pureconfig.error.ConfigReaderFailures

import scala.util.control.NoStackTrace

/**
 * WasmVm settings.
 *
 * @param defaultMaxMemPages The maximum number of memory pages when a module doesn't say
 * @param specTestRegister If true, registers the spec test harness as 'spectest'.
 */
case class VmConfig(
  defaultMaxMemPages: Int,
  specTestRegister: Boolean
)

object VmConfig {

  implicit def hint[T]: ProductHint[T] = ProductHint[T](ConfigFieldMapping(CamelCase, CamelCase))

  case class ConfigError(failures: ConfigReaderFailures) extends NoStackTrace

}
