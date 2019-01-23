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

import fluence.ethclient.helpers.Web3jConverters.bytes32ToBinary
import org.web3j.abi.datatypes.generated._
import scodec.bits.ByteVector

/** WorkerNode contains information describing a Worker as a Tendermint node
 * @param validatorKey 32-byte Tendermint Validator key, also represented by base64ValidatorKey
 * @param peerId hex-encoded 20-byte Tendermint peer ID which is calculated as `hex.EncodeToString(SHA256(peer.PubKey)[:20])`
 * and can be retrieved from Tendermint via command `show_node_id`
 * @param p2pPort p2p Tendermint port, used by Tendermint to connect p2p peers. Also used for rpcPort calculation
 * @param index index of a worker in cluster workers array
 */
case class WorkerNode(validatorKey: ByteVector, peerId: String, ip: InetAddress, p2pPort: Short, index: Int) {
  val base64ValidatorKey: String = validatorKey.toBase64
  val address: String = s"${ip.getHostAddress}:$p2pPort"
  val peerAddress: String = s"$peerId@$address"

  val rpcPort: Short = WorkerNode.rpcPort(p2pPort)
  val tmPrometheusPort: Short = WorkerNode.tmPrometheusPort(p2pPort)
  val smPrometheusPort: Short = WorkerNode.smPrometheusPort(p2pPort)
}

object WorkerNode {

  /** Build WorkerNode from web3 data structures
   * @param validatorKey Tendermint validator key, determines the node that controls the worker
   * @param nodeAddress is a concatenation of tendermint p2p node_id (20 bytes) and IPv4 address (4 bytes)
   * @param p2pPort Tendermint p2p port of the worker
   * @param index index of a worker in cluster workers array
   * @return WorkerNode instance
   */
  def apply(validatorKey: Bytes32, nodeAddress: Bytes24, p2pPort: Uint16, index: Int): WorkerNode = {
    val peerId = ByteVector(nodeAddress.getValue, 0, 20).toHex
    val ipBytes: Array[Byte] = ByteVector(nodeAddress.getValue, 20, 4).toArray.map(x => (x & 0xFF).toByte)
    val inetAddress = InetAddress.getByAddress(ipBytes)
    val portShort = p2pPort.getValue.shortValue()
    val keyBytes = bytes32ToBinary(validatorKey)

    WorkerNode(keyBytes, peerId, inetAddress, portShort, index)
  }

  //TODO: find a better way to calculate all these ports. Maybe Kademlia?
  def rpcPort(p2pPort: Short): Short = (p2pPort + 100).toShort
  def tmPrometheusPort(p2pPort: Short): Short = (p2pPort + 200).toShort
  def smPrometheusPort(p2pPort: Short): Short = (p2pPort + 300).toShort
}
