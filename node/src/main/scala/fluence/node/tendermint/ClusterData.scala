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
import fluence.node.tendermint.json.{Cluster, Genesis, NodeInfo}
import fluence.node.workers.CodePath
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated.{Bytes24, Bytes32, Uint16, Uint256}

import scodec.bits.ByteVector

import scala.collection.JavaConverters._

/**
 * All the information required to launch a worker.
 *
 * @param nodeInfo information about the tendermint node
 * @param nodePeer node persistent peer address
 * @param code code ID
 */
case class ClusterData(
  nodeInfo: NodeInfo,
  private[tendermint] val nodePeer: PersistentPeer,
  code: CodePath
) {
  val p2pPort: Short = nodePeer.port
  val rpcPort: Short = (p2pPort + 100).toShort //TODO: reserve service ports sequentially, right after p2p port
  val rpcHost: String = nodePeer.host
  val tmPrometheusPort: Short = (p2pPort + 200).toShort //TODO: reserve service ports sequentially, right after p2p port
  val smPrometheusPort: Short = (p2pPort + 300).toShort //TODO: reserve service ports sequentially, right after p2p port

  def clusterName: String = nodeInfo.clusterName

  def nodeIndex: Int = nodeInfo.node_index.toInt
}

object ClusterData {

  /**
   * Information about Tendermint peers.
   *
   * @param peers peer list
   */
  private case class PersistentPeers(peers: Vector[PersistentPeer]) {

    /**
     * Obtains Tendermint-compatible comma-separater peer list.
     */
    override def toString: String = peers.mkString(",")

    /**
     * Obtains external addresses (host:port) from peer list.
     */
    def externalAddrs: Seq[String] = peers.map(x => x.host + ":" + x.port)
  }

  /**
   * Obtains persistent peers from their encoded web3j's representation.
   *
   * @param addrs web3j's array of encoded addresses
   * @param ports web3j's array of ports
   */
  private def peersFromAddrsAndPorts(addrs: DynamicArray[Bytes24], ports: DynamicArray[Uint16]): PersistentPeers =
    PersistentPeers(
      addrs.getValue.asScala
        .map(_.getValue)
        .zip(ports.getValue.asScala.map(_.getValue))
        .map(
          x =>
            PersistentPeer(
              ByteVector(x._1, 0, 20).toHex,
              ByteVector(x._1, 20, 4).toArray.map(x => (x & 0xFF).toString).mkString("."),
              x._2.shortValue()
          )
        )
        .toVector
    )

  /**
   * Builds a ClusterData for a given data and NodeConfig. Removes ClusterData where the node does not participate.
   *
   * @param clusterID Cluster ID
   * @param nodeIds IDs of the nodes which participate in the cluster, order matters
   * @param genesisTime Unix timestamp for genesis block
   * @param storageHash Hash for the app in Swarm
   * @param nodeAddrs Addresses of the nodes in the same order as IDs
   * @param workerPorts Worker parts on the nodes in the same order as nodes and IDs
   * @param nodeConfig Current node's config
   * @return Some ClusterData if this node participates and data is well formed, None otherwise
   */
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
      val persistentPeers = peersFromAddrsAndPorts(nodeAddrs, workerPorts)
      val cluster = Cluster(genesis, persistentPeers.toString, persistentPeers.externalAddrs)
      val nodeInfo = NodeInfo(cluster, nodeIndex.toString)

      persistentPeers.peers
        .lift(nodeInfo.node_index.toInt)
        .map(
          ClusterData(nodeInfo, _, storage)
        )
    }
  }
}
