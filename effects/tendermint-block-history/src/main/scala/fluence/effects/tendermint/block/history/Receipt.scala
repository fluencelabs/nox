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

package fluence.effects.tendermint.block.history

import java.nio.ByteBuffer

import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}
import scodec.bits.ByteVector
import fluence.effects.tendermint.block.history.helpers.ByteVectorJsonCodec._

import scala.language.higherKinds

/**
 * Decentralized storage receipt
 *
 * @param hash Hash of the stored data
 */
case class Receipt(height: Long, hash: ByteVector) {

  def bytes(): ByteVector = {
    import io.circe.syntax._
    ByteVector((this: Receipt).asJson.noSpaces.getBytes())
  }

  def bytesCompact(): Array[Byte] = ByteBuffer.allocate(8).putLong(height).array() ++ hash.toArray
}

object Receipt {

  def fromBytesCompact(bytes: Array[Byte]): Receipt = {
    val (height, hash) = bytes.splitAt(8)
    // TODO: handle error?
    Receipt(ByteBuffer.wrap(height).getLong, ByteVector(hash))
  }

  implicit val dec: Decoder[Receipt] = deriveDecoder[Receipt]
  implicit val enc: Encoder[Receipt] = deriveEncoder[Receipt]
}
