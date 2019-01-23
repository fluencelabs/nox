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

import fluence.node.eth.conf.{EthereumRpcConfig, FluenceContractConfig}
import fluence.node.workers.tendermint.ValidatorKey
import fluence.node.workers.WorkerImage
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

/**
 * Main config class for master node.
 *
 * @param tendermintPath a path to all system files
 * @param endpoints information about a node possible endpoints (IP and ports) that will be used as addresses
 *                  for requests after a cluster will be formed
 * @param contract information about Fluence smart contract
 * @param swarm information about Swarm node
 * @param statusServer information about master node status server
 */
case class MasterConfig(
  tendermintPath: String,
  endpoints: EndpointsConfig,
  contract: FluenceContractConfig,
  swarm: Option[SwarmConfig],
  statusServer: StatusServerConfig,
  masterContainerId: Option[String],
  worker: WorkerImage,
  ethereum: EthereumRpcConfig
)

/**
 * @param host address to Swarm node
 */
case class SwarmConfig(host: String)

/**
 * @param port endpoint to master node status server
 */
case class StatusServerConfig(port: Int)

object MasterConfig {
  implicit val encodeEthereumConfig: Encoder[EthereumRpcConfig] = deriveEncoder
  implicit val decodeEthereumConfig: Decoder[EthereumRpcConfig] = deriveDecoder
  implicit val encodeContractConfig: Encoder[FluenceContractConfig] = deriveEncoder
  implicit val decodeContractConfig: Decoder[FluenceContractConfig] = deriveDecoder
  implicit val encodeSwarmConfig: Encoder[SwarmConfig] = deriveEncoder
  implicit val decodeSwarmConfig: Decoder[SwarmConfig] = deriveDecoder
  implicit val encodeStatConfig: Encoder[StatusServerConfig] = deriveEncoder
  implicit val decodeStatConfig: Decoder[StatusServerConfig] = deriveDecoder
  implicit val encodeMasterConfig: Encoder[MasterConfig] = deriveEncoder
  implicit val decodeMasterConfig: Decoder[MasterConfig] = deriveDecoder
}

/**
 * Information about a node possible endpoints (IP and ports) that will be used as addresses
 * for requests after a cluster will be formed
 *
 * @param ip p2p host IP
 * @param minPort starting port for p2p port range
 * @param maxPort ending port for p2p port range
  **/
case class EndpointsConfig(
  ip: InetAddress,
  minPort: Short,
  maxPort: Short
)

object EndpointsConfig {
  implicit val encodeInetAddress: Encoder[InetAddress] = Encoder[String].contramap(_.getHostAddress)
  implicit val decodeInetAddress: Decoder[InetAddress] = Decoder[String].map(InetAddress.getByName)
  implicit val encodeEndpointConfig: Encoder[EndpointsConfig] = deriveEncoder
  implicit val decodeEndpointConfig: Decoder[EndpointsConfig] = deriveDecoder
}

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
  workerImage: WorkerImage,
  isPrivate: Boolean = false
)

object NodeConfig {
  implicit val encodeNodeConfig: Encoder[NodeConfig] = deriveEncoder
  implicit val decodeNodeConfig: Decoder[NodeConfig] = deriveDecoder
}
