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

import fluence.kad.RoutingConf
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

import scala.concurrent.duration._

case class KademliaConfig(
  routing: RoutingConf,
  advertize: KademliaConfig.Advertize,
  join: KademliaConfig.Join
)

object KademliaConfig {
  case class Advertize(host: String, port: Short)

  case class Join(seeds: Seq[String], numOfNodes: Int)

  implicit val encodeAdvertize: Encoder[Advertize] = deriveEncoder
  implicit val decodeAdvertize: Decoder[Advertize] = deriveDecoder

  implicit val encodeJoin: Encoder[Join] = deriveEncoder
  implicit val decodeJoin: Decoder[Join] = deriveDecoder

  implicit val encodeDuration: Encoder[Duration] =
    Encoder.encodeDuration.contramap(d ⇒ java.time.Duration.ofMillis(d.toMillis))
  implicit val decodeDuration: Decoder[Duration] =
    Decoder.decodeDuration.map(_.toMillis.millis)

  implicit val encodeRoutingRefreshingConf: Encoder[RoutingConf.Refreshing] = deriveEncoder
  implicit val decodeRoutingRefreshingConf: Decoder[RoutingConf.Refreshing] = deriveDecoder

  implicit val encodeRoutingConf: Encoder[RoutingConf] = deriveEncoder
  implicit val decodeRoutingConf: Decoder[RoutingConf] = deriveDecoder

  implicit val encodeKademliaConfig: Encoder[KademliaConfig] = deriveEncoder
  implicit val decodeKademliaConfig: Decoder[KademliaConfig] = deriveDecoder
}
