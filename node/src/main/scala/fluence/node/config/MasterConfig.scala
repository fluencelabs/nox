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

import cats.effect.{ContextShift, IO}
import fluence.node.eth.DeployerContractConfig
import fluence.node.tendermint.{KeysPath, ValidatorKey}
import io.circe.{Encoder, Json}
import io.circe.generic.semiauto.deriveEncoder

/**
 * Main config class for master node.
 *
 * @param tendermintPath a path to all system files
 * @param endpoints information about a node possible endpoints (IP and ports) that will be used as addresses
 *                  for requests after a cluster will be formed
 * @param deployer information about deployer smart contract
 * @param swarm information about Swarm node
 * @param statServer information about master node status server
 */
case class MasterConfig(
  tendermintPath: String,
  endpoints: EndpointsConfig,
  deployer: DeployerContractConfig,
  swarm: Option[SwarmConfig],
  statServer: StatServerConfig
)

/**
 * @param host address to Swarm node
 */
case class SwarmConfig(host: String)

/**
 * @param port endpoint to master node status server
 */
case class StatServerConfig(port: Int)

object MasterConfig {
  implicit val encodeThrowable: Encoder[InetAddress] = new Encoder[InetAddress] {
    final def apply(a: InetAddress): Json = Json.fromString(a.getHostAddress)
  }
  implicit val encodeEndpointConfig: Encoder[EndpointsConfig] = deriveEncoder
  implicit val encodeDeployerConfig: Encoder[DeployerContractConfig] = deriveEncoder
  implicit val encodeSwarmConfig: Encoder[SwarmConfig] = deriveEncoder
  implicit val encodeStatConfig: Encoder[StatServerConfig] = deriveEncoder
  implicit val encodeMasterConfig: Encoder[MasterConfig] = deriveEncoder
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

/**
 * Information about a node willing to run solvers to join Fluence clusters.
 *
 * @param endpoints information about a node possible endpoints (IP and ports) that will be used as addresses
 *                 for requests after a cluster will be formed
 * @param validatorKey p2p port
 * @param nodeAddress p2p port
 */
case class NodeConfig(
  endpoints: EndpointsConfig,
  validatorKey: ValidatorKey,
  nodeAddress: String
)

object NodeConfig extends slogging.LazyLogging {
  private val MaxPortCount = 100
  private val MinPortCount = 0
  private val MinPort = 20000
  private val MaxPort = 40000
  private def MaxPort(range: Int = 0): Int = MaxPort - range

  /**
   * Builds [[NodeConfig]].
   *
   */
  def apply(keysPath: KeysPath, endpointsConfig: EndpointsConfig)(implicit ec: ContextShift[IO]): IO[NodeConfig] =
    for {
      validatorKey ← keysPath.showValidatorKey
      nodeAddress ← keysPath.showNodeId

      _ = logger.info("Tendermint node id: {}", nodeAddress.trim)

    } yield NodeConfig(endpointsConfig, validatorKey, nodeAddress)

  private def checkPorts(startPort: Int, endPort: Int): IO[Unit] = {
    val ports = endPort - startPort

    if (ports <= MinPortCount || ports > MaxPortCount) {
      IO.raiseError(
        new IllegalArgumentException(
          s"Port range size should be between $MinPortCount and $MaxPortCount"
        )
      )
    } else if (startPort < MinPort || startPort > MaxPort(ports) && endPort > MaxPort) {
      IO.raiseError(
        new IllegalArgumentException(
          s"Allowed ports should be between $MinPort and $MaxPort"
        )
      )
    } else {
      IO.unit
    }
  }

}
