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

package fluence.node.config.storage

import cats.syntax.either._
import com.softwaremill.sttp._
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}

import scala.concurrent.duration.FiniteDuration
import scala.util.Try

/**
 * Configuration for Swarm storage, part of [[RemoteStorageConfig]]
 *
 * @param address URI of a Swarm node
 */
case class SwarmConfig(address: Uri, readTimeout: FiniteDuration)

object SwarmConfig {
  import fluence.node.config.DurationCodecs._

  implicit val uriEncoder: Encoder[Uri] = Encoder.encodeString.contramap[Uri](_.toString)
  implicit val decodeUri: Decoder[Uri] =
    Decoder.decodeString.emap { str =>
      Try(uri"$str").toEither.leftMap(e => s"Error while decoding uri in SwarmConfig: $e")
    }
  implicit val encodeSwarmConfig: Encoder[SwarmConfig] = deriveEncoder
  implicit val decodeSwarmConfig: Decoder[SwarmConfig] = deriveDecoder
}
