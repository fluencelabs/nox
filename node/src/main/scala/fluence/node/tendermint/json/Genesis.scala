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

package fluence.node.tendermint.json

import java.text.SimpleDateFormat
import java.util.{Base64, Calendar, TimeZone}

import fluence.ethclient.helpers.Web3jConverters.bytes32AppIdToChainId
import io.circe.Encoder
import io.circe.generic.semiauto.deriveEncoder
import org.web3j.abi.datatypes.DynamicArray
import org.web3j.abi.datatypes.generated.{Bytes32, Uint256}

import scala.collection.JavaConverters._

/**
 * Cluster's genesis information in Tendermint-compatible format.
 *
 * @param genesis_time genesis time in yyyy-MM-dd'T'HH:mm:ss.SSS'Z' format
 * @param chain_id unique ID of the cluster
 * @param app_hash initial app hash
 * @param validators validator information
 */
case class Genesis(
  genesis_time: String,
  chain_id: String,
  app_hash: String,
  validators: Seq[Validator]
)

object Genesis {
  implicit val genesisEncoder: Encoder[Genesis] = deriveEncoder[Genesis]

  /**
   * Constructs Tendermint genesis from data obtained from contract event.
   *
   * @param appId encoded cluster ID
   * @param ids encoded Tendermint public key
   * @param genesisTimeUint256 encoded genesis time
   */
  def fromClusterData(
    appId: Bytes32,
    ids: DynamicArray[Bytes32],
    genesisTimeUint256: Uint256
  ): Genesis = {
    val validators = ids.getValue.asScala.zipWithIndex.map {
      case (x, i) =>
        Validator(
          ValidatorKey(
            "tendermint/PubKeyEd25519",
            Base64.getEncoder.encodeToString(x.getValue)
          ),
          "1",
          "node" + i
        )
    }.toArray

    val chainId = bytes32AppIdToChainId(appId)

    val calendar: Calendar = Calendar.getInstance
    calendar.setTimeInMillis(genesisTimeUint256.getValue.longValue() * 1000)

    val dateFormat = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'")
    dateFormat.setTimeZone(TimeZone.getTimeZone("UTC"))
    val genesisTime = dateFormat.format(calendar.getTime)

    Genesis(genesisTime, chainId, "", validators)
  }
}
