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

package fluence.node.eth.state

import java.net.InetAddress

import fluence.effects.ethclient.helpers.Web3jConverters.bytes32ToBinary
import org.web3j.abi.datatypes.generated._
import scodec.bits.ByteVector

/**
 * WorkerPeer contains information describing a Worker as a Tendermint node
 *
 * @param validatorKey 32-byte Tendermint Validator key, also represented by base64ValidatorKey
 * @param peerId hex-encoded 20-byte Tendermint peer ID which is calculated as `hex.EncodeToString(SHA256(peer.PubKey)[:20])`
 *               and can be retrieved from Tendermint via command `show_node_id`
 * @param apiPort Remote peer's API port
 * @param index index of a worker in cluster workers array
 */
case class WorkerPeer(validatorKey: ByteVector, peerId: String, apiPort: Short, ip: InetAddress, index: Int) {
  val base64ValidatorKey: String = validatorKey.toBase64

  def address(p2pPort: Short): String = s"${ip.getHostAddress}:$p2pPort"

  def peerAddress(p2pPort: Short): String = s"$peerId@${address(p2pPort)}"
}

object WorkerPeer {

  /**
   * Build WorkerNode from web3 data structures
   *
   * @param validatorKey Tendermint validator key, determines the node that controls the worker
   * @param nodeAddress is a concatenation of tendermint p2p node_id (20 bytes) and IPv4 address (4 bytes)
   * @param index index of a worker in cluster workers array
   * @return WorkerPeer instance
   */
  def apply(validatorKey: Bytes32, nodeAddress: Bytes24, apiPort: Short, index: Int): WorkerPeer = {
    val peerId = ByteVector(nodeAddress.getValue, 0, 20).toHex
    val ipBytes: Array[Byte] = ByteVector(nodeAddress.getValue, 20, 4).toArray.map(x => (x & 0xFF).toByte)
    val inetAddress = InetAddress.getByAddress(ipBytes)
    val keyBytes = bytes32ToBinary(validatorKey)

    WorkerPeer(keyBytes, peerId, apiPort, inetAddress, index)
  }
}
