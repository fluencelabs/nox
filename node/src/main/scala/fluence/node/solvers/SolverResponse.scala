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

package fluence.node.solvers
import io.circe.{Decoder, Encoder}
import io.circe.generic.extras.Configuration
import io.circe.generic.semiauto._

/**
 * Status information from a tendermint node.
 */
object SolverResponse {

  case class OtherInfo(
    amino_version: String,
    p2p_version: String,
    consensus_version: String,
    rpc_version: String,
    tx_index: String,
    rpc_address: String
  )

  case class NodeInfo(
    id: String,
    listen_addr: String,
    network: String,
    version: String,
    channels: String,
    moniker: String,
    other: OtherInfo
  )

  case class SyncInfo(
    latest_block_hash: String,
    latest_app_hash: String,
    latest_block_height: Int,
    latest_block_time: String,
    catching_up: Boolean
  )

  case class PubKey(`type`: String, value: String)

  case class ValidatorInfo(address: String, pub_key: PubKey, voting_power: String)

  case class SolverTendermintInfo(node_info: NodeInfo, sync_info: SyncInfo, validator_info: ValidatorInfo)

  case class SolverResponse(result: SolverTendermintInfo)

  implicit val configuration: Configuration =
    Configuration.default.withSnakeCaseMemberNames.withSnakeCaseConstructorNames

  implicit val decodeOtherInfo: Decoder[OtherInfo] = deriveDecoder
  implicit val decodeNodeInfo: Decoder[NodeInfo] = deriveDecoder
  implicit val decodeSyncInfo: Decoder[SyncInfo] = deriveDecoder
  implicit val decodePubKey: Decoder[PubKey] = deriveDecoder
  implicit val decodeValidatorInfo: Decoder[ValidatorInfo] = deriveDecoder
  implicit val decodeCheck: Decoder[SolverTendermintInfo] = deriveDecoder
  implicit val decodeResponse: Decoder[SolverResponse] = deriveDecoder

  implicit val encodeOtherInfo: Encoder[OtherInfo] = deriveEncoder
  implicit val encodeNodeInfo: Encoder[NodeInfo] = deriveEncoder
  implicit val encodeSyncInfo: Encoder[SyncInfo] = deriveEncoder
  implicit val encodePubKey: Encoder[PubKey] = deriveEncoder
  implicit val encodeValidatorInfo: Encoder[ValidatorInfo] = deriveEncoder
  implicit val encodeCheck: Encoder[SolverTendermintInfo] = deriveEncoder
  implicit val encodeResponse: Encoder[SolverResponse] = deriveEncoder
}
