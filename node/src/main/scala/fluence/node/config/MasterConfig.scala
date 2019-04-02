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
import fluence.node.config.LogLevel.LogLevel
import fluence.node.config.storage.RemoteStorageConfig
import fluence.node.workers.tendermint.config.TendermintConfig
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}
import net.ceedubs.ficus.readers.ValueReader

import scala.util.Try

/**
 * Main config class for master node.
 *
 * @param rootPath A path to all node's files, including its Tendermint keys and all the Apps with codes and Tendermint data
 * @param endpoints Information about a node possible endpoints (IP and ports) that will be used as addresses
 *                  for requests after a cluster will be formed
 * @param contract Information about Fluence smart contract
 * @param remoteStorage Configuration for remote decentralized content-addressable stores
 * @param httpApi Information about master node status server
 */
case class MasterConfig(
  rootPath: String,
  ports: PortsConfig,
  endpoints: EndpointsConfig,
  contract: FluenceContractConfig,
  remoteStorage: RemoteStorageConfig,
  httpApi: HttpApiConfig,
  masterContainerId: Option[String],
  worker: DockerConfig,
  tendermint: DockerConfig,
  ethereum: EthereumRpcConfig,
  tendermintConfig: TendermintConfig,
  logLevel: LogLevel
)

object MasterConfig {
  import ConfigOps._
  import net.ceedubs.ficus.readers.namemappers.implicits.hyphenCase
  import net.ceedubs.ficus.readers.ArbitraryTypeReader._
  import net.ceedubs.ficus.readers.EnumerationReader._
  import net.ceedubs.ficus.Ficus._
  import net.ceedubs.ficus.readers.ArbitraryTypeReader._

  implicit val encodeMasterConfig: Encoder[MasterConfig] = deriveEncoder
  implicit val decodeMasterConfig: Decoder[MasterConfig] = deriveDecoder
  implicit val shortReader: ValueReader[Short] = ValueReader[Int].map(_.toShort)

  def load(): IO[MasterConfig] =
    ConfigOps.loadConfigAs[MasterConfig]()
}
