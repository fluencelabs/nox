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

package fluence.effects.tendermint.block.data

import fluence.effects.tendermint.block.protobuf.ProtobufJson
import io.circe.Decoder
import io.circe.generic.extras.Configuration
import io.circe.generic.extras.semiauto.deriveDecoder
import proto3.tendermint.{BlockID, Vote}
import scodec.bits.ByteVector

/**
 * Provides Json decoders that reencode some of the byte fields in the following manner: base64 encode -> hex decode -> to byte string
 * This is to work around Tendermint's amino JSON serialization quirk. For more details, see [[ProtobufJson.reencode]]
 */
object ReencodingJsonCodecs {
  import SimpleJsonCodecs.Decoders.{
    base64ByteVectorDecoder,
    byteVectorDecoder,
    dataDecoder,
    timestampDecoder,
    versionDecoder
  }
  implicit val conf: Configuration = Configuration.default.withDefaults

  implicit final val voteDecoder: Decoder[Vote] =
    Decoder.decodeJson.emap(jvalue => ProtobufJson.voteReencoded(jvalue).left.map(e => s"Vote: $e"))

  implicit final val blockIdDecoder: Decoder[BlockID] =
    Decoder.decodeJson.emap(ProtobufJson.blockIdReencoded(_).left.map(e => s"BlockID: $e"))

  implicit final val lastCommitDecoder: Decoder[LastCommit] = deriveDecoder[LastCommit]

  implicit final val headerDecoder: Decoder[Header] = deriveDecoder[Header]

  implicit final val blockDecoder: Decoder[Block] = deriveDecoder[Block]

}
