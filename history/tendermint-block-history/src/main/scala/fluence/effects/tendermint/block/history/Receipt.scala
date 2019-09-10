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

import cats.kernel.Semigroup
import fluence.codec.{CodecError, PureCodec}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}
import scodec.bits.ByteVector
import fluence.effects.tendermint.block.history.helpers.ByteVectorJsonCodec._

import scala.language.higherKinds
import scala.util.Try

/**
 * Decentralized storage receipt
 *
 * @param hash Hash of the stored data
 */
case class Receipt(height: Long, hash: ByteVector) {

  lazy val jsonString: String = {
    import io.circe.syntax._
    (this: Receipt).asJson.noSpaces
  }

  def jsonBytes(): ByteVector =
    ByteVector(jsonString.getBytes())

  def bytesCompact(): Array[Byte] =
    (ByteVector.fromLong(height) ++ hash).toArray
}

object Receipt {

  def fromBytesCompact(bytes: Array[Byte]): Either[Throwable, Receipt] = {
    val (height, hash) = bytes.splitAt(8)
    Try(ByteVector(height).toLong()).toEither.map(Receipt(_, ByteVector(hash)))
  }

  implicit val dec: Decoder[Receipt] = deriveDecoder[Receipt]
  implicit val enc: Encoder[Receipt] = deriveEncoder[Receipt]

  implicit val pureReceiptCodec: PureCodec[Receipt, Array[Byte]] =
    PureCodec.build(
      PureCodec.liftFunc[Receipt, Array[Byte]](
        _.bytesCompact()
      ),
      PureCodec.liftFuncEither[Array[Byte], Receipt](
        fromBytesCompact(_).left.map(t ⇒ CodecError("Cannot decode receipt from bytes compact", Some(t)))
      )
    )

  implicit val ReceiptSemigroup: Semigroup[Receipt] =
    (x, y) ⇒ if (x.height > y.height) x else y
}
