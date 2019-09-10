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

import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import scodec.bits.ByteVector

/**
 * Block receipt, should be provided by Node for each non-empty block
 *
 * @param height Corresponding block's height
 * @param bytes Receipt bytes to form the state hash
 */
case class BlockReceipt(height: Long, bytes: ByteVector)

object BlockReceipt {
  private implicit val decbc: Decoder[ByteVector] =
    Decoder.decodeString.map(_.getBytes()).map(ByteVector(_))

  private implicit val encbc: Encoder[ByteVector] =
    Encoder.encodeString.contramap(bv ⇒ new String(bv.toArray))

  implicit val dec: Decoder[BlockReceipt] = deriveDecoder[BlockReceipt]
  implicit val enc: Encoder[BlockReceipt] = deriveEncoder[BlockReceipt]
}
