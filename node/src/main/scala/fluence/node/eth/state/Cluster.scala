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

import fluence.ethclient.helpers.Web3jConverters.bytes32ToBinary
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated._

import scala.collection.JavaConverters._
import scala.concurrent.duration.{FiniteDuration, _}

/**
 * Represents a Fluence cluster
 *
 * @param genesisTime Unix timestamp of cluster creation, used for Tendermint genesis.json config generation
 * @param workers List of members of this cluster, also contain `currentWorker`
 * @param currentWorker A worker belonging to current Fluence node
 */
case class Cluster private[eth] (genesisTime: FiniteDuration, workers: Vector[WorkerPeer], currentWorker: WorkerPeer)

object Cluster {

  /**
   * Builds a Cluster structure, filters for clusters that include current node.
   * i.e., return None if validatorKeys doesn't contain currentValidatorKey
   *
   * @param time Cluster genesis time
   * @param validatorKeys array of 32-byte Tendermint validator keys, used as workers' ids
   * @param ports array of ports for Node's API connection
   * @param currentValidatorKey 32-byte Tendermint validator key corresponding to current node
   */
  private[eth] def build(
    time: Uint256,
    validatorKeys: DynamicArray[Bytes32],
    nodeAddresses: DynamicArray[Bytes24],
    ports: DynamicArray[Uint16],
    currentValidatorKey: Bytes32,
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
          // TODO check that port is correct Short
          WorkerPeer(validator, address, port.getValue.shortValueExact(), idx)
      }
      .toVector

    val keyBytes = bytes32ToBinary(currentValidatorKey)
    val currentWorker = workers.find(_.validatorKey === keyBytes)
    currentWorker.map(cw => Cluster(timestamp, workers, cw))
  }
}
