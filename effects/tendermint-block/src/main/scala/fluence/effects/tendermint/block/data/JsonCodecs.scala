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
import io.circe.generic.extras.Configuration
import io.circe.generic.extras.semiauto.deriveDecoder
import io.circe.{Decoder, Encoder}
import proto3.tendermint.{BlockID, Version, Vote}
import scalapb.GeneratedMessage
import scalapb_circe.JsonFormat
import scodec.bits.ByteVector

object JsonCodecs {
  /*  Config  */
  implicit val conf: Configuration = Configuration.default.withDefaults

  /*  Encoders  */
  implicit def messageEncoder[A <: GeneratedMessage]: Encoder[A] = Encoder.instance(JsonFormat.toJson)
  implicit final val byteVectorEncoder: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)

  /*  Decoders  */
  implicit final val base64ByteVectorDecoder: Decoder[Base64ByteVector] = Decoder.decodeString.emap(
    str => ByteVector.fromBase64Descriptive(str).map(Base64ByteVector).left.map(_ => "Base64ByteVector")
  )

  implicit final val voteDecoder: Decoder[Vote] =
    Decoder.decodeJson.emap(jvalue => ProtobufJson.vote(jvalue).left.map(_ => "Vote"))

  implicit final val byteVectorDecoder: Decoder[ByteVector] = {
    Decoder.decodeString.emap { str =>
      ByteVector.fromHexDescriptive(str).left.map(_ => "ByteVector")
    }
  }

  implicit final val versionDecoder: Decoder[Version] = {
    Decoder.decodeJson.emap { jvalue =>
      ProtobufJson.version(jvalue).left.map(_ => "Version")
    }
  }

  implicit final val timestampDecoder: Decoder[Timestamp] = {
    Decoder.decodeJson.emap(jvalue => ProtobufJson.timestamp(jvalue).left.map(_ => "Timestamp"))
  }

  implicit final val blockIdDecoder: Decoder[BlockID] = {
    Decoder.decodeJson.emap { jvalue =>
      ProtobufJson.blockId(jvalue).left.map(_ => "BlockID")
    }
  }

  implicit final val dataDecoder: Decoder[Data] = deriveDecoder
}
