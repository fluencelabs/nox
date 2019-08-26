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

package fluence.node.workers.tendermint

import cats.Id
import fluence.crypto.KeyPair
import fluence.crypto.eddsa.Ed25519
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import scodec.bits.ByteVector

/**
 * Representations of node_key.json file
 *
 * @param priv_key Private key
 */
case class TendermintNodeKey(priv_key: TendermintNodeKey.PrivKey) {

  /**
   * Transform raw tendermint key to KeyPair.
   *
   */
  def getKeyPair: Either[String, KeyPair] =
    for {
      privKey <- ByteVector.fromBase64Descriptive(priv_key.value)
      secret = KeyPair.Secret(privKey)
      kp <- Ed25519.ed25519.restorePairFromSecret[Id](secret).value.left.map(_.message)
    } yield kp

}

object TendermintNodeKey {
  case class PrivKey(`type`: String, value: String)

  implicit val privKeyDecoder: Decoder[PrivKey] = deriveDecoder[PrivKey]
  implicit val privKeyEncoder: Encoder[PrivKey] = deriveEncoder[PrivKey]

  implicit val nodeKeyDecoder: Decoder[TendermintNodeKey] = deriveDecoder[TendermintNodeKey]
  implicit val nodeKeyEncoder: Encoder[TendermintNodeKey] = deriveEncoder[TendermintNodeKey]

}
