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
import java.util.{Base64, Calendar, Date}

import fluence.ethclient.Deployer.ClusterFormedEventResponse
import fluence.ethclient.data._
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated.Bytes32
import scodec.bits.{Bases, ByteVector}
import io.circe.generic.auto._
import io.circe.syntax._

import scala.collection.JavaConverters._

object Web3jConverters {

  def stringToBytes32(s: String): Bytes32 = {
    val byteValue = s.getBytes()
    val byteValueLen32 = new Array[Byte](32)
    System.arraycopy(byteValue, 0, byteValueLen32, 0, byteValue.length)
    new Bytes32(byteValueLen32)
  }

  def stringToHex(s: String): String = binaryToHex(s.getBytes())

  def binaryToHex(b: Array[Byte]): String = ByteVector(b).toHex

  def base64ToBytes32(b64: String): Bytes32 = new Bytes32(Base64.getDecoder.decode(b64))

  def b32ToChainId(clusterId: Bytes32): String = binaryToHex(clusterId.getValue.reverse.take(1)) // TODO:

  def b32DAtoGenesis(clusterId: Bytes32, ids: DynamicArray[Bytes32]): TendermintGenesis = {
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

    val chainId = b32ToChainId(clusterId)
    //val genesisTime = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'").format(Calendar.getInstance().getTime)
    val genesisTime = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'").format(new Date(2018 - 1900, 11 - 1, 3))
    TendermintGenesis(genesisTime, chainId, "", validators)
  }

  def hexToArrayUnsafe(hex: String): Array[Byte] =
    ByteVector.fromHex(hex, Bases.Alphabets.HexUppercase).map(_.toArray).getOrElse(new Array[Byte](hex.length / 2))

  def solverAddressToBytes32(ip: String, port: Short, nodeIdHex: String): Bytes32 = {
    val buffer = new Array[Byte](32)

    Array.copy(hexToArrayUnsafe(nodeIdHex), 0, buffer, 0, 20)
    Array.copy(ip.split('.').map(_.toInt.toByte), 0, buffer, 20, 4)
    buffer(24) = ((port >> 8) & 0xFF).toByte
    buffer(25) = (port & 0xFF).toByte

    new Bytes32(buffer)
  }

  def b32DAtoPersistentPeers(peers: DynamicArray[Bytes32]): PersistentPeers =
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

  def clusterFormedEventToClusterData(
    event: ClusterFormedEventResponse,
    nodeKey: TendermintValidatorKey
  ): Option[ClusterData] = {
    val genesis = b32DAtoGenesis(event.clusterID, event.solverIDs)
    val nodeIndex = genesis.validators.indexWhere(_.pub_key == nodeKey)
    println("found " + nodeIndex + " for " + nodeKey.asJson.noSpaces + " in " + genesis.asJson.noSpaces)
    if (nodeIndex == -1)
      None
    else {
      val persistentPeers = b32DAtoPersistentPeers(event.solverAddrs)
      val cluster = Cluster(genesis, persistentPeers.toString, persistentPeers.externalAddrs)
      val nodeInfo = NodeInfo(cluster, nodeIndex.toString)
      Some(ClusterData(nodeInfo, persistentPeers, "llamadb"))
    }
  }

}
