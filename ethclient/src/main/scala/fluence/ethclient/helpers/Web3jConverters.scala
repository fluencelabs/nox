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
import java.util.{Base64, Calendar, TimeZone}

import fluence.ethclient.Deployer.ClusterFormedEventResponse
import fluence.ethclient.data._
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated._
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
   * @param genesisTimeUint256 encoded genesis time
   */
  def clusterDataToGenesis(
    clusterId: Bytes32,
    ids: DynamicArray[Bytes32],
    genesisTimeUint256: Uint256
  ): TendermintGenesis = {
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
    calendar.setTimeInMillis(genesisTimeUint256.getValue.longValue() * 1000)

    val dateFormat = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'")
    dateFormat.setTimeZone(TimeZone.getTimeZone("UTC"))
    val genesisTime = dateFormat.format(calendar.getTime)

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
   * @param nodeAddressHex Tendermint p2p public key
   */
  def solverAddressToBytes24(ip: String, nodeAddressHex: String): Bytes24 = {
    val buffer = new Array[Byte](24)

    Array.copy(hexToBinary(nodeAddressHex), 0, buffer, 0, 20)
    Array.copy(ip.split('.').map(_.toInt.toByte), 0, buffer, 20, 4)

    new Bytes24(buffer)
  }

  /**
   * Obtains persistent peers from their encoded web3j's representation.
   *
   * @param addrs web3j's array of encoded addresses
   * @param ports web3j's array of ports
   */
  def addrsAndPortsToPersistentPeers(addrs: DynamicArray[Bytes24], ports: DynamicArray[Uint16]): PersistentPeers =
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
        .toArray
    )

  /**
   * Tries to convert `ClusterFormedEvent` response to [[ClusterData]] with all information to launch cluster.
   *
   * @param event event response
   * @param solverInfo information about current solver
   * @return true if provided node key belongs to the cluster from the event
   */
  def clusterFormedEventToClusterData(
    event: ClusterFormedEventResponse,
    solverInfo: SolverInfo
  ): Option[ClusterData] = {
    val genesis = clusterDataToGenesis(event.clusterID, event.solverIDs, event.genesisTime)
    val nodeIndex = genesis.validators.indexWhere(_.pub_key == solverInfo.validatorKey)
    if (nodeIndex == -1)
      None
    else {
      val storageHash = bytes32ToString(event.storageHash) // TODO: temporarily used as name of pre-existing local code
      val persistentPeers = addrsAndPortsToPersistentPeers(event.solverAddrs, event.solverPorts)
      val cluster = Cluster(genesis, persistentPeers.toString, persistentPeers.externalAddrs)
      val nodeInfo = NodeInfo(cluster, nodeIndex.toString)
      Some(ClusterData(nodeInfo, persistentPeers, storageHash, solverInfo.longTermLocation))
    }
  }

}
