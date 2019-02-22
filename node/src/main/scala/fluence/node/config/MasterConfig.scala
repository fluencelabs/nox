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

import cats.effect.IO
import fluence.node.docker.DockerImage
import fluence.node.workers.tendermint.config.TendermintConfig
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}
import net.ceedubs.ficus.readers.ValueReader

/**
 * Main config class for master node.
 *
 * @param rootPath a path to all node's files, including its Tendermint keys and all the Apps with codes and Tendermint data
 * @param endpoints information about a node possible endpoints (IP and ports) that will be used as addresses
 *                  for requests after a cluster will be formed
 * @param contract information about Fluence smart contract
 * @param swarm information about Swarm node
 * @param httpApi information about master node status server
 */
case class MasterConfig(
  rootPath: String,
  ports: PortsConfig,
  endpoints: EndpointsConfig,
  contract: FluenceContractConfig,
  swarm: SwarmConfig,
  httpApi: HttpApiConfig,
  masterContainerId: Option[String],
  worker: DockerImage,
  tendermint: DockerImage,
  ethereum: EthereumRpcConfig,
  tendermintConfig: TendermintConfig
)

object MasterConfig {

  implicit val encodeMasterConfig: Encoder[MasterConfig] = deriveEncoder
  implicit val decodeMasterConfig: Decoder[MasterConfig] = deriveDecoder

  import ConfigOps.inetAddressValueReader
  import net.ceedubs.ficus.readers.ArbitraryTypeReader._
  import net.ceedubs.ficus.Ficus._

  implicit val shortReader: ValueReader[Short] = ValueReader[Int].map(_.toShort)

  // TODO it could be easily done with more stable ConfigFactory.load, having application.conf in ENV
  def load(): IO[MasterConfig] =
    ConfigOps.loadConfigAs[MasterConfig]()
}
