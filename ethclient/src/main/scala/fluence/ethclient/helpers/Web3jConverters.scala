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

package fluence.ethclient.helpers
import java.text.SimpleDateFormat
import java.util.{Base64, Calendar}

import fluence.ethclient.Deployer.ClusterFormedEventResponse
import fluence.ethclient.data._
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated.{Bytes32, Int64}
import scodec.bits.{Bases, ByteVector}

import scala.collection.JavaConverters._

object Web3jConverters {

  /**
   * Converts string to web3j's Bytes32.
   *
   * @param s string
   */
  def stringToBytes32(s: String): Bytes32 = {
    val byteValue = s.getBytes()
    val byteValueLen32 = new Array[Byte](32)
    System.arraycopy(byteValue, 0, byteValueLen32, 0, byteValue.length)
    new Bytes32(byteValueLen32)
  }

  /**
   * Converts string to hex.
   *
   * @param s string
   */
  def stringToHex(s: String): String = binaryToHex(s.getBytes())

  /**
   * Converts byte array to hex.
   *
   * @param b byte array
   */
  def binaryToHex(b: Array[Byte]): String = ByteVector(b).toHex

  /**
   * Converts base64 string to web3j's Bytes32.
   *
   * @param base64 base64 string
   */
  def base64ToBytes32(base64: String): Bytes32 = new Bytes32(Base64.getDecoder.decode(base64))

  /**
   * Interprets web3j's Bytes32 as Tendermint chain ID.
   * TODO: currently only the lowermost byte used
   *
   * @param clusterId Bytes32 encoding
   */
  def bytes32ClusterIdToChainId(clusterId: Bytes32): String = binaryToHex(clusterId.getValue.reverse.take(1))

  /**
   * Converts non-zero bytes of web3j's Bytes32 to string.
   *
   * @param bytes32 text in Bytes32 encoding
   */
  def bytes32ToString(bytes32: Bytes32): String = new String(bytes32.getValue.filter(_ != 0))

  /**
   * Constructs Tendermint genesis from data obtained from contract event.
   *
   * @param clusterId encoded cluster ID
   * @param ids encoded Tendermint public key
   * @param genesisTimeI64 encoded genesis time
   */
  def clusterDataToGenesis(clusterId: Bytes32, ids: DynamicArray[Bytes32], genesisTimeI64: Int64): TendermintGenesis = {
    val validators = ids.getValue.asScala.zipWithIndex.map {
      case (x, i) =>
        TendermintValidator(
          TendermintValidatorKey(
            "tendermint/PubKeyEd25519",
            Base64.getEncoder.encodeToString(x.getValue)
          ),
          "1",
          "node" + i
        )
    }.toArray

    val chainId = bytes32ClusterIdToChainId(clusterId)

    val calendar: Calendar = Calendar.getInstance
    // TODO: figure out why Tendermint sometimes treat genesis time as 'future' and remove time decrement
    calendar.setTimeInMillis(genesisTimeI64.getValue.longValue() - 1000000)
    val genesisTime = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'").format(calendar.getTime)
    TendermintGenesis(genesisTime, chainId, "", validators)
  }

  /**
   * Converts hex string to byte array.
   * TODO: add checks, now it's unsafe.
   *
   * @param hex hex string
   */
  def hexToBinary(hex: String): Array[Byte] =
    ByteVector.fromHex(hex, Bases.Alphabets.HexUppercase).map(_.toArray).getOrElse(new Array[Byte](hex.length / 2))

  /**
   * Encodes solver address information to web3j's Bytes32.
   *
   * @param ip solver host IP
   * @param port solver p2p port
   * @param nodeIdHex Tendermint p2p public key
   */
  def solverAddressToBytes32(ip: String, port: Short, nodeIdHex: String): Bytes32 = {
    val buffer = new Array[Byte](32)

    Array.copy(hexToBinary(nodeIdHex), 0, buffer, 0, 20)
    Array.copy(ip.split('.').map(_.toInt.toByte), 0, buffer, 20, 4)
    buffer(24) = ((port >> 8) & 0xFF).toByte
    buffer(25) = (port & 0xFF).toByte

    new Bytes32(buffer)
  }

  /**
   * Obtains persistent peers from their encoded web3j's representation.
   *
   * @param peers web3j's array of encoded addresses
   */
  def bytes32DynamicArrayToPersistentPeers(peers: DynamicArray[Bytes32]): PersistentPeers =
    PersistentPeers(
      peers.getValue.asScala
        .map(_.getValue)
        .map(
          x =>
            PersistentPeer(
              ByteVector(x, 0, 20).toHex,
              ByteVector(x, 20, 4).toArray.map(x => (x & 0xFF).toString).mkString("."),
              (((x(24) & 0xFF) << 8) + (x(25) & 0xFF)).toShort
          )
        )
        .toArray
    )

  /**
   * Tries to convert `ClusterFormedEvent` response to [[ClusterData]] with all information to launch cluster.
   *
   * @param event event response
   * @param nodeKey node key from launching data used to obtain solver's index
   * @return true if provided node key belongs to the cluster from the event
   */
  def clusterFormedEventToClusterData(
    event: ClusterFormedEventResponse,
    nodeKey: TendermintValidatorKey
  ): Option[ClusterData] = {
    val genesis = clusterDataToGenesis(event.clusterID, event.solverIDs, event.genesisTime)
    val nodeIndex = genesis.validators.indexWhere(_.pub_key == nodeKey)
    if (nodeIndex == -1)
      None
    else {
      val storageHash = bytes32ToString(event.storageHash) // TODO: temporarily used as name of pre-existing local code
      val persistentPeers = bytes32DynamicArrayToPersistentPeers(event.solverAddrs)
      val cluster = Cluster(genesis, persistentPeers.toString, persistentPeers.externalAddrs)
      val nodeInfo = NodeInfo(cluster, nodeIndex.toString)
      Some(ClusterData(nodeInfo, persistentPeers, storageHash))
    }
  }

}
