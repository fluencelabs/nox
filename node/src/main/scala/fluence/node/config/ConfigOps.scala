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
import java.net.InetAddress

import cats.effect.IO
import com.softwaremill.sttp._
import com.typesafe.config.{Config, ConfigFactory}
import net.ceedubs.ficus.readers.ValueReader
import net.ceedubs.ficus.Ficus._

private[config] object ConfigOps {

  def loadConfigAs[T: ValueReader](conf: ⇒ Config = ConfigFactory.load()): IO[T] =
    IO(conf.as[T])

  implicit val inetAddressValueReader: ValueReader[InetAddress] =
    ValueReader[String].map(InetAddress.getByName)

  implicit val uriReader: ValueReader[Uri] =
    ValueReader[String].map(s => uri"$s")
}
