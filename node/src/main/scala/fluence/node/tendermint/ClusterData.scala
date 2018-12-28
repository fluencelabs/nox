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
import fluence.node.config.NodeConfig
import fluence.node.solvers.CodePath
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated.{Bytes24, Bytes32, Uint16, Uint256}

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
  code: CodePath
) {
  private val nodePeer = persistentPeers.peers(nodeInfo.node_index.toInt) //TODO: catch OutOfBounds exception
  val p2pPort: Short = nodePeer.port
  val rpcPort: Short = (p2pPort + 100).toShort //TODO: reserve service ports sequentially, right after p2p port
  val rpcHost: String = nodePeer.host
  val tmPrometheusPort: Short = (p2pPort + 200).toShort //TODO: reserve service ports sequentially, right after p2p port
  val smPrometheusPort: Short = (p2pPort + 300).toShort //TODO: reserve service ports sequentially, right after p2p port

  def clusterName: String = nodeInfo.clusterName

  def nodeIndex: Int = nodeInfo.node_index.toInt
}

object ClusterData {

  def build(
    clusterID: Bytes32,
    nodeIds: DynamicArray[Bytes32],
    genesisTime: Uint256,
    storageHash: Bytes32,
    nodeAddrs: DynamicArray[Bytes24],
    workerPorts: DynamicArray[Uint16],
    nodeConfig: NodeConfig
  ): Option[ClusterData] = {
    val genesis = Genesis.fromClusterData(clusterID, nodeIds, genesisTime)
    val nodeIndex = genesis.validators.indexWhere(_.pub_key == nodeConfig.validatorKey)
    if (nodeIndex == -1)
      None
    else {
      val storage = CodePath(storageHash)
      val persistentPeers = PersistentPeers.fromAddrsAndPorts(nodeAddrs, workerPorts)
      val cluster = Cluster(genesis, persistentPeers.toString, persistentPeers.externalAddrs)
      val nodeInfo = NodeInfo(cluster, nodeIndex.toString)
      Some(ClusterData(nodeInfo, persistentPeers, storage))
    }
  }
}
