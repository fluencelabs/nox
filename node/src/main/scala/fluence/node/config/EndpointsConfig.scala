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

import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

/**
 * Information about a node possible endpoints (IP and ports) that will be used as addresses
 * for requests after a cluster will be formed
 *
 * @param ip p2p host IP
  **/
case class EndpointsConfig(ip: InetAddress)

object EndpointsConfig {
  private implicit val encodeInetAddress: Encoder[InetAddress] = Encoder[String].contramap(_.getHostAddress)
  private implicit val decodeInetAddress: Decoder[InetAddress] = Decoder[String].map(InetAddress.getByName)
  implicit val encodeEndpointConfig: Encoder[EndpointsConfig] = deriveEncoder
  implicit val decodeEndpointConfig: Decoder[EndpointsConfig] = deriveDecoder
}
