package fluence.ethclient.data

case class ClusterInfo(
  clusterInfoJson: String,
  code: String,
  chainId: String,
  nodeIndex: Int,
  hostP2pPort: Short,
  hostRpcPort: Short,
  tmPrometheusPort: Short,
  smPrometheusPort: Short
)
