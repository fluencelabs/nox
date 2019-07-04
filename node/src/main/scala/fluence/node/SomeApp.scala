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

package fluence.node

import fluence.node.workers.tendermint.TendermintPrivateKey
import io.circe.parser.decode
import scodec.bits.ByteVector

object SomeApp extends App {

  val a =
    """
      |{
      |          "address": "C08269A8AACD53C3488F16F285821DAC77CF5DEF",
      |          "pub_key": {
      |            "type": "tendermint/PubKeyEd25519",
      |            "value": "FWB5lXZ/TT2132+jXp/8aQzNwISwp9uuFz4z0TXDdxY="
      |          },
      |          "priv_key": {
      |            "type": "tendermint/PrivKeyEd25519",
      |            "value": "P6jw9q/Rytdxpv5Wxs1aYA8w82uS0x3CpmS9+GpaMGIVYHmVdn9NPbXfb6Nen/xpDM3AhLCn264XPjPRNcN3Fg=="
      |          }
      |        }
      |""".stripMargin
  val parsed = decode[TendermintPrivateKey](a).right.get
  val keys = TendermintPrivateKey.getKeyPair(parsed).right.get

  val b58 = "HYAXgtDyiuLzeGR4A7j5g8gYX4Poezt845PbanxeF32H"
  val pubKeySome = ByteVector.fromBase58Descriptive(b58).right.get
  println(pubKeySome.toBase64)

  println(keys)
}
