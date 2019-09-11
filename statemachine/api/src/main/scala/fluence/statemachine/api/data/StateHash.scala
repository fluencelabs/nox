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

package fluence.statemachine.api.data

import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}
import scodec.bits.ByteVector

/**
 * Statemachine's state after a certain block.
 * Hash is formed from the previous [[BlockReceipt]] and VM hash.
 *
 * @param height Block's height
 * @param hash State hash
 */
case class StateHash(height: Long, hash: ByteVector)

object StateHash {
  val empty = StateHash(0, ByteVector.empty)

  private implicit val decbc: Decoder[ByteVector] =
    Decoder.decodeString.flatMap(
      ByteVector.fromHex(_).fold(Decoder.failedWithMessage[ByteVector]("Not a hex"))(Decoder.const)
    )
  private implicit val encbc: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)

  implicit val enc: Encoder[StateHash] = deriveEncoder[StateHash]
  implicit val dec: Decoder[StateHash] = deriveDecoder[StateHash]
}
