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

import Settings.CodeDirectory
import cats.effect.IO
import fluence.statemachine.config.StateMachineConfig
import fluence.statemachine.error.VmModuleLocationError
import slogging.MessageFormatter.DefaultPrefixFormatter
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

object Utils {

  def getWasmFiles() =
    StateMachineConfig
      .listWasmFiles(CodeDirectory)
      .map(
        _.toNel.toRight(
          new RuntimeException(
            VmModuleLocationError("Provided directories don't contain any wasm or wast files").toString
          )
        )
      )
      .flatMap(IO.fromEither)

  def configureLogging(level: LogLevel): Unit = {
    PrintLoggerFactory.formatter =
      new DefaultPrefixFormatter(printLevel = true, printName = true, printTimestamp = true)
    LoggerConfig.factory = PrintLoggerFactory()
    LoggerConfig.level = level
  }
}
