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
import fluence.node.docker.DockerImage
import fluence.node.workers.tendermint.ValidatorKey
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

/**
 * Information about a node willing to run workers to join Fluence clusters.
 *
 * @param endpoints information about a node possible endpoints (IP and ports) that will be used as addresses
 *                 for requests after a cluster will be formed
 * @param validatorKey Tendermint validator public key, used by node for participation in Tendermint consensus
 * @param nodeAddress p2p ID for this node. Basically first 20 bytes of p2p peer SHA256(PubKey)
 */
case class NodeConfig(
  endpoints: EndpointsConfig,
  validatorKey: ValidatorKey,
  nodeAddress: String,
  workerImage: DockerImage,
  isPrivate: Boolean = false
)

object NodeConfig {
  implicit val encodeNodeConfig: Encoder[NodeConfig] = deriveEncoder
  implicit val decodeNodeConfig: Decoder[NodeConfig] = deriveDecoder
}
