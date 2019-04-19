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

package fluence.effects.tendermint.block

import com.google.protobuf.ByteString
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.Decoder
import proto3.tendermint.{BlockID, Vote}
import scodec.bits.ByteVector

import scala.util.Try

object Header {
  implicit final val decodeByteVector: Decoder[ByteVector] = {
    Decoder.decodeString.emap { str =>
      ByteVector.fromHexDescriptive(str).left.map(_ => "ByteVector")
    }
  }

  implicit val decodeVersion: Decoder[proto3.tendermint.Version] = {
    Decoder.decodeJson.emap { jvalue =>
      Try(JSON.version(jvalue)).toEither.left.map(_ => "Version")
    }
  }

  implicit val decodeTimestamp: Decoder[com.google.protobuf.timestamp.Timestamp] = {
    Decoder.decodeJson.emap(jvalue => Try(JSON.timestamp(jvalue)).toEither.left.map(_ => "Timestamp"))
  }

  implicit val decodeBlockID: Decoder[BlockID] = {
    Decoder.decodeJson.emap { jvalue =>
      Try(JSON.blockId(jvalue)).toEither.left.map(_ => "BlockID")
    }
  }

  implicit val headerDecoder: Decoder[Header] = deriveDecoder[Header]
}

case class Header(
  version: Option[proto3.tendermint.Version],
  chain_id: String,
  height: Long,
  time: Option[com.google.protobuf.timestamp.Timestamp],
  num_txs: Long,
  total_txs: Long,
  last_block_id: Option[BlockID],
  last_commit_hash: ByteVector,
  data_hash: ByteVector,
  validators_hash: ByteVector,
  next_validators_hash: ByteVector,
  consensus_hash: ByteVector,
  app_hash: ByteVector,
  last_results_hash: ByteVector,
  evidence_hash: ByteVector,
  proposer_address: ByteVector,
)
