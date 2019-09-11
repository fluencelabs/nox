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

package fluence.statemachine.abci.peers

import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import scodec.bits.ByteVector

/**
 * A signal to drop Tendermint validator by setting it's voting power to zero
 * Used to build a Tendermint's ValidatorUpdate command
 * see https://github.com/tendermint/tendermint/blob/master/docs/spec/abci/abci.md#validatorupdate
 *
 * @param validatorKey Validator key's bytes
 */
case class DropPeer(validatorKey: ByteVector)

object DropPeer {
  // Type of a Tendermint node's validator key. Currently always PubKeyEd25519.
  val KeyType = "PubKeyEd25519"

  implicit val dec: Decoder[DropPeer] = deriveDecoder[DropPeer]
  private implicit val decbc: Decoder[ByteVector] =
    Decoder.decodeString.flatMap(
      ByteVector.fromHex(_).fold(Decoder.failedWithMessage[ByteVector]("Not a hex"))(Decoder.const)
    )

  implicit val enc: Encoder[DropPeer] = deriveEncoder[DropPeer]
  private implicit val encbc: Encoder[ByteVector] = Encoder.encodeString.contramap(_.toHex)
}
