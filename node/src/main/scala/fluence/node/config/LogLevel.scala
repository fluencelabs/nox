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

package fluence.node.config
import io.circe.{Decoder, Encoder}

import scala.language.implicitConversions

object LogLevel extends Enumeration {
  type LogLevel = Value

  val OFF = Value("OFF")
  val ERROR = Value("ERROR")
  val WARN = Value("WARN")
  val INFO = Value("INFO")
  val DEBUG = Value("DEBUG")
  val TRACE = Value("TRACE")

  implicit def toSlogging(l: LogLevel): slogging.LogLevel = {
    l match {
      case `OFF` => slogging.LogLevel.OFF
      case `ERROR` => slogging.LogLevel.ERROR
      case `WARN` => slogging.LogLevel.WARN
      case `INFO` => slogging.LogLevel.INFO
      case `DEBUG` => slogging.LogLevel.DEBUG
      case `TRACE` => slogging.LogLevel.TRACE
    }
  }

  implicit val dec: Decoder[LogLevel.Value] = Decoder.enumDecoder(LogLevel)
  implicit val enc: Encoder[LogLevel.Value] = Encoder.enumEncoder(LogLevel)
}
