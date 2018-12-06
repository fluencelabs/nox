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

package fluence.node.tendermint
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated.{Bytes24, Uint16}
import scodec.bits.ByteVector

import scala.collection.JavaConverters._

/**
 * Information about Tendermint peers.
 *
 * @param peers peer list
 */
case class PersistentPeers(peers: Seq[PersistentPeer]) {

  /**
   * Obtains Tendermint-compatible comma-separater peer list.
   */
  override def toString: String = peers.mkString(",")

  /**
   * Obtains external addresses (host:port) from peer list.
   */
  def externalAddrs: Seq[String] = peers.map(x => x.host + ":" + x.port)
}

object PersistentPeers {

  /**
   * Obtains persistent peers from their encoded web3j's representation.
   *
   * @param addrs web3j's array of encoded addresses
   * @param ports web3j's array of ports
   */
  def fromAddrsAndPorts(addrs: DynamicArray[Bytes24], ports: DynamicArray[Uint16]): PersistentPeers =
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
    )
}
