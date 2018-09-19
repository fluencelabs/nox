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

package fluence.swarm.helpers
import io.circe.{Decoder, Encoder, Json}
import scodec.bits.ByteVector

object ByteVectorJsonCodec {

  /**
   * Every byte array in JSON is `0x` prefixed in Swarm.
   */
  implicit final val encodeByteVector: Encoder[ByteVector] = (bv: ByteVector) => Json.fromString("0x" + bv.toHex)

  implicit final val decodeByteVector: Decoder[ByteVector] = {
    Decoder.decodeString.emap { str =>
      ByteVector.fromHexDescriptive(str).left.map(t => "ByteVector")
    }
  }
}
