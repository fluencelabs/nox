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

sealed case class NodeInfo(cluster: Cluster, node_index: String)

sealed case class Cluster(genesis: TendermintGenesis, persistent_peers: String, external_addrs: Array[String])

sealed case class TendermintGenesis(genesis_time: String, chain_id: String, app_hash: String, validators: Array[TendermintValidator])

sealed case class TendermintValidator(pub_key: TendermintValidatorKey, power: String, name: String)

sealed case class TendermintValidatorKey(`type`: String, value: String)

sealed case class PersistentPeers(peers: Array[PersistentPeer]) {
  override def toString: String = peers.mkString(",")

  def externalAddrs: Array[String] = peers.map(x => x.host + ":" + x.port)
}

sealed case class PersistentPeer(address: String, host: String, port: Short) {
  override def toString: String = s"$address@$host:$port"
}
