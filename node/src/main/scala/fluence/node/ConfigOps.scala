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

package fluence.node

import java.net.InetAddress

import cats.effect.IO
import pureconfig.ConfigConvert
import pureconfig.error.ConfigReaderFailures
import pureconfig.ConfigConvert.viaStringTry

import scala.util.Try

object ConfigOps {
  implicit class ConfigLoaderToIO[T](loadedConfig: Either[ConfigReaderFailures, T]) {

    def toIO: IO[T] = {
      IO.fromEither(
        loadedConfig.left.map(
          fs =>
            new IllegalArgumentException(
              "Can't load or parse configs:\n" + fs.toList.map(f => f.location + " - " + f.description).mkString("\n")
          )
        )
      )

    }

  }

  implicit val inetAddressConvert: ConfigConvert[InetAddress] =
    viaStringTry[InetAddress](str => Try(InetAddress.getByName(str)), _.toString)
}
