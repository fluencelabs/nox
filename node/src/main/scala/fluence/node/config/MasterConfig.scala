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
import Configuration.loadConfig
import fluence.node.docker.DockerImage
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
  worker: DockerImage,
  ethereum: EthereumRpcConfig
)

object MasterConfig {
  implicit val encodeMasterConfig: Encoder[MasterConfig] = deriveEncoder
  implicit val decodeMasterConfig: Decoder[MasterConfig] = deriveDecoder

  import pureconfig.generic.auto._
  import ConfigOps._

  def load(): IO[MasterConfig] =
    for {
      config <- loadConfig()
      masterConfig <- pureconfig.loadConfig[MasterConfig](config).toIO
    } yield masterConfig
}
