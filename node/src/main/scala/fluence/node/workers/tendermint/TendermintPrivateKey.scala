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

case class PubKey(`type`: String, value: String)
case class PrivKey(`type`: String, value: String)

/**
 * Representations of priv_validator_key.json file
 *
 * @param priv_key Private key + public key in base64 format
 * @param pub_key Public key in base64 format
 */
case class TendermintPrivateKey(priv_key: PrivKey, pub_key: Option[PubKey])

object TendermintPrivateKey {
  implicit val privKeyDecoder: Decoder[PrivKey] = deriveDecoder[PrivKey]
  implicit val privKeyEncoder: Encoder[PrivKey] = deriveEncoder[PrivKey]

  implicit val pubKeyDecoder: Decoder[PubKey] = deriveDecoder[PubKey]
  implicit val pubKeyEncoder: Encoder[PubKey] = deriveEncoder[PubKey]

  implicit val validatorKeyDecoder: Decoder[TendermintPrivateKey] = deriveDecoder[TendermintPrivateKey]
  implicit val validatorKeyEncoder: Encoder[TendermintPrivateKey] = deriveEncoder[TendermintPrivateKey]

  /**
   * Transform raw tendermint key to KeyPair.
   *
   */
  def getKeyPair(tendermintKey: TendermintPrivateKey): Either[String, KeyPair] = {
    for {
      privKey <- ByteVector.fromBase64Descriptive(tendermintKey.priv_key.value)
      secret = KeyPair.Secret(privKey.dropRight(32))
      pubKey <- tendermintKey.pub_key
        .map(_.value)
        .fold(
          Ed25519.ed25519.restorePairFromSecret[Id](secret).value.left.map(_.message).map(_.publicKey)
        )(ByteVector.fromBase64Descriptive(_).map(KeyPair.Public))
    } yield KeyPair(pubKey, secret)
  }
}
