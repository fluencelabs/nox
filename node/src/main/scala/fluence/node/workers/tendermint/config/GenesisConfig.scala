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

package fluence.node.workers.tendermint.config

private[config] object GenesisConfig {
  import java.text.SimpleDateFormat
  import java.util.TimeZone

  import fluence.ethclient.helpers.Web3jConverters
  import fluence.node.eth.state.App
  import fluence.node.workers.tendermint.ValidatorKey
  import io.circe.Encoder
  import io.circe.generic.semiauto.deriveEncoder
  import io.circe.syntax._

  private case class ValidatorConfig(pub_key: ValidatorKey, power: String, name: String)
  private case class GenesisConfig(
    genesis_time: String,
    chain_id: String,
    app_hash: String,
    validators: Seq[ValidatorConfig]
  )

  def generateJson(app: App): String = {
    val dateFormat = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'")
    dateFormat.setTimeZone(TimeZone.getTimeZone("UTC"))

    GenesisConfig(
      genesis_time = dateFormat.format(app.cluster.genesisTime.toMillis),
      chain_id = Web3jConverters.appIdToChainId(app.id),
      app_hash = "",
      validators = app.cluster.workers.map { w =>
        ValidatorConfig(
          ValidatorKey(
            `type` = "tendermint/PubKeyEd25519",
            value = w.base64ValidatorKey
          ),
          power = "1",
          name = s"${app.id}_${w.index}"
        )
      }
    ).asJson.spaces2
  }

  implicit private val configEncoder: Encoder[GenesisConfig] = deriveEncoder
  implicit private val validatorEncoder: Encoder[ValidatorConfig] = deriveEncoder
}
