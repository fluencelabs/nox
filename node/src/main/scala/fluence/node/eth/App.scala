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

package fluence.node.eth
import java.net.InetAddress
import java.text.SimpleDateFormat
import java.util.{Calendar, TimeZone}

import org.web3j.abi.datatypes.{Address, DynamicArray}
import org.web3j.abi.datatypes.generated._
import scodec.bits.ByteVector

import scala.concurrent.duration.FiniteDuration
import scala.concurrent.duration._
import scala.collection.JavaConverters._
import fluence.ethclient.helpers.Web3jConverters.{bytes32ToBase64, bytes32ToHexString}

case class App(
  appId: Bytes32,
  storageHash: Bytes32,
  cluster: Cluster //TODO: maybe make cluster an Option
) {
  def appIdHex: String = bytes32ToHexString(appId)
}

object App {
  def apply(appId: Bytes32, storageHash: Bytes32, cluster: Cluster): App = App(appId, storageHash, cluster)
}

case class Cluster(genesisTime: FiniteDuration, workers: List[WorkerNode], currentWorker: WorkerNode)

object Cluster {

  /* Builds a Cluster structure, filters for clusters that include current node.
   * i.e., return None if validatorKeys doesn't contain currentWorkerId
   * @param time Cluster genesis time
   * @param validatorKeys array of 32-byte Tendermint validator keys, used as workers' ids
   * @param ports array of ports for Tendermint p2p connection. For rpc and prometheus ports see [[WorkerNode]]
   * @param currentWorkerId 32-byte Tendermint validator key corresponding to current node
   */
  def build(
    time: Uint256,
    validatorKeys: DynamicArray[Bytes32],
    nodeAddresses: DynamicArray[Bytes24],
    ports: DynamicArray[Uint16],
    currentWorkerId: Bytes32,
  ): Option[Cluster] = {
    val timestamp = (time.getValue.longValue() * 1000).millis
    val keys = validatorKeys.getValue.asScala
    val addresses = nodeAddresses.getValue.asScala
    val ps = ports.getValue.asScala

    val workers = keys
      .zip(addresses)
      .zip(ps)
      .zipWithIndex
      .map {
        case (((validator, address), port), idx) =>
          WorkerNode(validator, address, port, idx)
      }
      .toList

    val currentWorker = workers.find(_.validatorKey == currentWorkerId)

    currentWorker.map(cw => Cluster(timestamp, workers, cw))
  }
}

/* WorkerNode contains information describing a Worker as a Tendermint node
 * @param workerId 32-byte Tendermint Validator key, also represented by base64WorkerId
 * @param peerId hex-encoded 20-byte Tendermint peer ID which is calculated as `hex.EncodeToString(SHA256(peer.PubKey)[:20])`
 * and can be retrieved from Tendermint via command `show_node_id`
 * @param port p2p Tendermint port, used by Tendermint to connect p2p peers. Also used for rpcPort calculation
 * @param index index of a worker in cluster workers array
 */
case class WorkerNode(validatorKey: Bytes32, peerId: String, ip: InetAddress, port: Short, index: Int) {
  def base64ValidatorKey: String = bytes32ToBase64(validatorKey)
  def peerAddress: String = s"$peerId@${ip.getHostAddress}:$port"

  val rpcPort: Short = (port + 100).toShort //TODO: reserve service ports sequentially, right after p2p port
  val tmPrometheusPort: Short = (port + 200).toShort //TODO: reserve service ports sequentially, right after p2p port
  val smPrometheusPort: Short = (port + 300).toShort //TODO: reserve service ports sequentially, right after p2p port
}

object WorkerNode {

  // @param nodeAddress is a concatenation of tendermint p2p node_id (20 bytes) and IPv4 address (4 bytes)
  def apply(key: Bytes32, nodeAddress: Bytes24, port: Uint16, index: Int): WorkerNode = {
    val peerId = ByteVector(nodeAddress.getValue, 0, 20).toHex
    val ipBytes: Array[Byte] = ByteVector(nodeAddress.getValue, 20, 4).toArray.map(x => (x & 0xFF).toByte)
    val inetAddress = InetAddress.getByAddress(ipBytes)
    val portShort = port.getValue.shortValue()

    WorkerNode(key, peerId, inetAddress, portShort, index)
  }
}
