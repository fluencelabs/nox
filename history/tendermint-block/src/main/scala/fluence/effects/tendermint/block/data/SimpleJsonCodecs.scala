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

import com.google.protobuf.timestamp.Timestamp
import fluence.effects.tendermint.block.protobuf.ProtobufJson
import io.circe.{Decoder, Encoder}
import io.circe.generic.extras.Configuration
import io.circe.generic.extras.semiauto.{deriveConfiguredDecoder, deriveEncoder}
import proto3.tendermint.{BlockID, Version, Vote}
import scalapb.GeneratedMessage
import scalapb_circe.JsonFormat
import scodec.bits.ByteVector

/**
 * As opposed to [[ReencodingJsonCodecs]], SimpleJsonCodecs provide decoders which
 * don't use [[fluence.effects.tendermint.block.protobuf.ProtobufJson.reencode]]
 */
object SimpleJsonCodecs {

  object Encoders {
    implicit val conf: Configuration = Configuration.default.withDefaults

    implicit def messageEncoder[A <: GeneratedMessage]: Encoder[A] = Encoder.instance(JsonFormat.toJson)

    implicit final val byteVectorEncoder: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)

    implicit final val base64ByteVectorEncoder: Encoder[Base64ByteVector] =
      Encoder.encodeString.contramap(_.bv.toBase64)

    implicit final val lastCommitEncoder: Encoder[LastCommit] = deriveEncoder

    implicit final val dataEncoder: Encoder[Data] = deriveEncoder

    implicit final val headerEncoder: Encoder[Header] = deriveEncoder[Header]

    implicit final val blockEncoder: Encoder[Block] = deriveEncoder
  }

  object Decoders {
    implicit val conf: Configuration = Configuration.default.withDefaults

    implicit final val base64ByteVectorDecoder: Decoder[Base64ByteVector] = Decoder.decodeString.emap(
      ByteVector.fromBase64Descriptive(_).map(Base64ByteVector).left.map(e => s"Base64ByteVector: $e")
    )

    implicit final val byteVectorDecoder: Decoder[ByteVector] =
      Decoder.decodeString.emap(ByteVector.fromHexDescriptive(_).left.map(e => s"ByteVector: $e"))

    implicit final val voteDecoder: Decoder[Vote] =
      Decoder.decodeJson.emap(ProtobufJson.voteSimple(_).left.map(e => s"Vote: $e"))

    implicit final val versionDecoder: Decoder[Version] =
      Decoder.decodeJson.emap(ProtobufJson.version(_).left.map(e => s"Version: $e"))

    implicit final val timestampDecoder: Decoder[Timestamp] =
      Decoder.decodeJson.emap(jvalue => ProtobufJson.timestamp(jvalue).left.map(e => s"Timestamp: $e"))

    implicit final val dataDecoder: Decoder[Data] = deriveConfiguredDecoder

    implicit final val blockIdDecoder: Decoder[BlockID] =
      Decoder.decodeJson.emap(ProtobufJson.blockIdSimple(_).left.map(e => s"BlockID: $e"))

    implicit final val lastCommitDecoder: Decoder[LastCommit] = deriveConfiguredDecoder

    implicit final val headerDecoder: Decoder[Header] = deriveConfiguredDecoder

    implicit final val blockDecoder: Decoder[fluence.effects.tendermint.block.data.Block] = deriveConfiguredDecoder
  }
}
