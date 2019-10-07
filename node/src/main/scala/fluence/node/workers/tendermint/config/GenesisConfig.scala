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

import java.nio.file.{Files, Path}
import java.text.SimpleDateFormat
import java.util.TimeZone

import cats.Monad
import cats.effect.{IO, LiftIO}
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.log.Log
import fluence.node.workers.tendermint.ValidatorPublicKey
import fluence.worker.eth.EthApp
import io.circe.Encoder
import io.circe.generic.semiauto.deriveEncoder

import scala.language.higherKinds

/**
 * Tendermint's genesis.json representation
 */
case class GenesisConfig private (
  genesis_time: String,
  chain_id: String,
  app_hash: String,
  validators: Seq[ValidatorConfig]
) {
  import GenesisConfig.configEncoder

  /**
   * Convert to canonical string representation
   */
  def toJsonString: String = configEncoder(this).spaces2

  /**
   * Write genesis.json inside `destPath`
   *
   * @param destPath Tendermint config directory to write genesis.json to
   */
  def writeTo[F[_]: Monad: LiftIO: Log](destPath: Path): F[Unit] =
    Log[F].info(s"Writing $destPath/genesis.json") >>
      IO(Files.write(destPath.resolve("genesis.json"), toJsonString.getBytes)).to[F].void
}

private object GenesisConfig {

  implicit val configEncoder: Encoder[GenesisConfig] = deriveEncoder

  /**
   * Prepare GenesisConfig for the given App
   */
  def apply(app: EthApp): GenesisConfig = {
    val dateFormat = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'")
    dateFormat.setTimeZone(TimeZone.getTimeZone("UTC"))

    GenesisConfig(
      genesis_time = dateFormat.format(app.cluster.genesisTime.toMillis),
      chain_id = app.id.toString,
      app_hash = "",
      validators = app.cluster.workers.map { w =>
        ValidatorConfig(
          ValidatorPublicKey(
            `type` = "tendermint/PubKeyEd25519",
            value = w.base64ValidatorKey
          ),
          power = "1",
          name = s"${app.id}_${w.index}"
        )
      }
    )
  }

}
