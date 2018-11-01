package fluence.ethclient.data

case class ClusterData(nodeInfo: NodeInfo, persistentPeers: PersistentPeers, code: String) {
  val hostP2pPort: Short = persistentPeers.peers(nodeInfo.node_index.toInt).port
  val hostRpcPort: Short = (hostP2pPort + 1).toShort
  val tmPrometheusPort: Short = (hostP2pPort + 4).toShort
  val smPrometheusPort: Short = (hostP2pPort + 5).toShort
}
