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

package fluence.statemachine.api.signals

import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}
import scodec.bits.ByteVector

/**
 * Common trait for all control signals from node to worker
 */
sealed trait ControlSignal

/**
 * Asks worker for it's status
 */
case class GetStatus() extends ControlSignal

object GetStatus {
  implicit val dec: Decoder[GetStatus] = deriveDecoder[GetStatus]
  implicit val enc: Encoder[GetStatus] = deriveEncoder[GetStatus]
}

/**
 * Tells worker to stop
 */
case class Stop() extends ControlSignal

object Stop {
  implicit val dec: Decoder[Stop] = deriveDecoder[Stop]
  implicit val enc: Encoder[Stop] = deriveEncoder[Stop]
}

/**
 * A signal to drop Tendermint validator by setting it's voting power to zero
 * Used to build a Tendermint's ValidatorUpdate command
 * see https://github.com/tendermint/tendermint/blob/master/docs/spec/abci/abci.md#validatorupdate
 *
 * @param validatorKey Validator key's bytes
 */
case class DropPeer(validatorKey: ByteVector) extends ControlSignal

object DropPeer {
  // Type of a Tendermint node's validator key. Currently always PubKeyEd25519.
  val KEY_TYPE = "PubKeyEd25519"

  implicit val dec: Decoder[DropPeer] = deriveDecoder[DropPeer]
  private implicit val decbc: Decoder[ByteVector] =
    Decoder.decodeString.flatMap(
      ByteVector.fromHex(_).fold(Decoder.failedWithMessage[ByteVector]("Not a hex"))(Decoder.const)
    )

  implicit val enc: Encoder[DropPeer] = deriveEncoder[DropPeer]
  private implicit val encbc: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)
}

case class BlockReceipt(height: Long, bytes: ByteVector) extends ControlSignal

object BlockReceipt {
  private implicit val decbc: Decoder[ByteVector] =
    Decoder.decodeString.map(_.getBytes()).map(ByteVector(_))

  private implicit val encbc: Encoder[ByteVector] =
    Encoder.encodeString.contramap(bv ⇒ new String(bv.toArray))

  implicit val dec: Decoder[BlockReceipt] = deriveDecoder[BlockReceipt]
  implicit val enc: Encoder[BlockReceipt] = deriveEncoder[BlockReceipt]
}

case class GetVmHash(height: Long) extends ControlSignal

object GetVmHash {
  implicit val dec: Decoder[GetVmHash] = deriveDecoder
  implicit val enc: Encoder[GetVmHash] = deriveEncoder
}
