package fluence.ethclient.data

sealed case class NodeInfo(cluster: Cluster, node_index: String)

sealed case class Cluster(genesis: Genesis, persistent_peers: String, external_addrs: Array[String])

sealed case class Genesis(genesis_time: String, chain_id: String, app_hash: String, validators: Array[Validator])

sealed case class Validator(pub_key: String, power: String, name: String)

sealed case class PersistentPeers(peers: Array[PersistentPeer]) {
  override def toString: String = peers.mkString(",")

  def externalAddrs: Array[String] = peers.map(_.address)
}

sealed case class PersistentPeer(address: String, host: String, port: Short) {
  override def toString: String = s"$address@$host:$port"
}