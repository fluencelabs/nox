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

package fluence.ethclient.data

// Currently there are several nested and partially duplicated structures (ClusterData -> NodeInfo -> Cluster).
// They are temporarily left to be compatible with `master-run-node.sh` script.
// TODO: refactor these structures.

/**
 * All the information required to launch solver.
 *
 * @param nodeInfo information about node in a format compatible with `master-run-node.sh` script
 * @param persistentPeers cluster peers information
 * @param code code ID
 * @param longTermLocation local directory with pre-initialized Tendermint public/private keys
 */
sealed case class ClusterData(
  nodeInfo: NodeInfo,
  persistentPeers: PersistentPeers,
  code: String,
  longTermLocation: String
) {
  val hostP2PPort: Short = persistentPeers.peers(nodeInfo.node_index.toInt).port
  val hostRpcPort: Short = (hostP2PPort + 100).toShort
  val tmPrometheusPort: Short = (hostP2PPort + 200).toShort
  val smPrometheusPort: Short = (hostP2PPort + 300).toShort
}

/**
 * Information about the cluster and current node. Compatible with `master-run-node.sh` script.
 *
 * @param cluster cluster information
 * @param node_index node index
 */
sealed case class NodeInfo(cluster: Cluster, node_index: String)

/**
 * Information about the cluster without node-specific data.
 *
 * @param genesis genesis in Tendermint format
 * @param persistent_peers p2p peers in Tendermint format
 * @param external_addrs external address used to initialize advertised address in launching scripts
 */
sealed case class Cluster(genesis: TendermintGenesis, persistent_peers: String, external_addrs: Array[String])

/**
 * Cluster's genesis information in Tendermint-compatible format.
 *
 * @param genesis_time genesis time in yyyy-MM-dd'T'HH:mm:ss.SSS'Z' format
 * @param chain_id unique ID of the cluster
 * @param app_hash initial app hash
 * @param validators validator information
 */
sealed case class TendermintGenesis(
  genesis_time: String,
  chain_id: String,
  app_hash: String,
  validators: Array[TendermintValidator]
)

/**
 * Cluster's solver (validator) information in Tendermint-compatible format.
 *
 * @param pub_key public key
 * @param power initial voting power
 * @param name validator name
 */
sealed case class TendermintValidator(pub_key: TendermintValidatorKey, power: String, name: String)

/**
 * Validator's public key in Tendermint-compatible format.
 *
 * @param `type` key type
 * @param value 32-byte public key in base64 representation
 */
sealed case class TendermintValidatorKey(`type`: String, value: String)

/**
 * Information about Tendermint peers.
 *
 * @param peers peer list
 */
sealed case class PersistentPeers(peers: Array[PersistentPeer]) {

  /**
   * Obtains Tendermint-compatible comma-separater peer list.
   */
  override def toString: String = peers.mkString(",")

  /**
   * Obtains external addresses (host:port) from peer list.
   */
  def externalAddrs: Array[String] = peers.map(x => x.host + ":" + x.port)
}

/**
 * Information about a single Tendermint peer
 *
 * @param address Tendermint address 20-byte key in hex representation
 * @param host host
 * @param port port
 */
sealed case class PersistentPeer(address: String, host: String, port: Short) {
  override def toString: String = s"$address@$host:$port"
}
