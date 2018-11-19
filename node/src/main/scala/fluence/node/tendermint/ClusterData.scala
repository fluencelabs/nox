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

package fluence.node.tendermint
import fluence.ethclient.Deployer.ClusterFormedEventResponse
import fluence.ethclient.helpers.Web3jConverters.bytes32ToString
import fluence.node.NodeConfig

/**
 * All the information required to launch solver.
 *
 * @param nodeInfo information about node in a format compatible with `master-run-node.sh` script
 * @param persistentPeers cluster peers information
 * @param code code ID
 */
case class ClusterData(
  nodeInfo: NodeInfo,
  persistentPeers: PersistentPeers,
  code: String
) {
  val hostP2PPort: Short = persistentPeers.peers(nodeInfo.node_index.toInt).port
  val hostRpcPort: Short = (hostP2PPort + 100).toShort
  val tmPrometheusPort: Short = (hostP2PPort + 200).toShort
  val smPrometheusPort: Short = (hostP2PPort + 300).toShort

  def clusterName: String = nodeInfo.clusterName

  def nodeIndex: Int = nodeInfo.node_index.toInt

  def nodeName: String = s"${clusterName}_node$nodeIndex"
}

object ClusterData {

  /**
   * Tries to convert `ClusterFormedEvent` response to [[ClusterData]] with all information to launch cluster.
   * TODO this method couples ClusterData with Ethereum's Deployer structure, consider moving it to DeployerContract instead
   *
   * @param event event response
   * @param nodeConfig information about current node
   * @return true if provided node key belongs to the cluster from the event
   */
  def fromClusterFormedEvent(
    event: ClusterFormedEventResponse,
    nodeConfig: NodeConfig
  ): Option[ClusterData] = {
    val genesis = Genesis.fromClusterData(event.clusterID, event.solverIDs, event.genesisTime)
    val nodeIndex = genesis.validators.indexWhere(_.pub_key == nodeConfig.validatorKey)
    if (nodeIndex == -1)
      None
    else {
      val storageHash = bytes32ToString(event.storageHash) // TODO: temporarily used as name of pre-existing local code
      val persistentPeers = PersistentPeers.fromAddrsAndPorts(event.solverAddrs, event.solverPorts)
      val cluster = Cluster(genesis, persistentPeers.toString, persistentPeers.externalAddrs)
      val nodeInfo = NodeInfo(cluster, nodeIndex.toString)
      Some(ClusterData(nodeInfo, persistentPeers, storageHash))
    }
  }
}
