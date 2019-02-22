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
import com.typesafe.config.ConfigObject
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import pureconfig.ConfigReader

/**
 * @param host address to Swarm node
 */
case class SwarmConfig(host: String)

object SwarmConfig {
  import pureconfig.generic.auto.exportReader

  // TODO remove this, use a simple boolean flag to enable/disable swarm, as this kills pureconfig compilation
  /**
   * Parse `swarm {}` as None.
   * WARNING: Config should always contain a `swarm` section, be it empty or with values inside.
   * TODO: Fix reader to treat absence of `swarm` section as None
   */
  implicit val swarmOptionalConfigReader: ConfigReader[Option[SwarmConfig]] =
    ConfigReader.fromCursor[Option[SwarmConfig]] { cv =>
      cv.value match {
        case co: ConfigObject if co.isEmpty => Right(None)
        case _ => ConfigReader[SwarmConfig].from(cv).map(Some(_))
      }
    }

  implicit val encodeSwarmConfig: Encoder[SwarmConfig] = deriveEncoder
  implicit val decodeSwarmConfig: Decoder[SwarmConfig] = deriveDecoder
}
