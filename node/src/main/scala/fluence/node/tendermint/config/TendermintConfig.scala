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

package fluence.node.tendermint.config
import java.nio.file.StandardCopyOption.REPLACE_EXISTING
import java.nio.file.{Files, Path, StandardCopyOption}
import java.text.SimpleDateFormat
import java.util.TimeZone

import cats.effect.IO
import fluence.ethclient.helpers.Web3jConverters
import fluence.node.Configuration
import fluence.node.config.NodeConfig
import fluence.node.eth.App
import fluence.node.workers.{CodeManager, CodePath, WorkerImage, WorkerParams}
import org.web3j.abi.datatypes.generated.Bytes32

import scala.io.Source

object TendermintConfig extends slogging.LazyLogging {

  /**
   * Generate, copy and/or update different configs used by tendermint and download vm code.
   *
   * `rootPath` is usually /master inside Master container
   * `templateConfigDir` contain:
   *    - configs generated by `tendermint --init` (see [[Configuration.tendermintInit]])
   *    - config/default_config.toml, copied on container build (see node's dockerfile in build.sbt)
   *
   * At the end of execution `workerPath` will contain:
   *    - vm code at `codePath`
   *    - tendermint configuration in `workerConfigDir`:
   *        - node_key.json, containing private P2P key
   *        - priv_validator.json, containing validator's private & public keys and it's address
   *        - genesis.json, generated from [[App.cluster]] and [[App.appId]]
   *        - config.toml, copied from `templateConfigDir/default_config.toml` and updated
   */
  def prepareWorkerParams(
    workerId: Bytes32,
    workerImage: WorkerImage,
    rootPath: Path,
    masterNodeContainerId: Option[String],
    codeManager: CodeManager[IO]
  ): fs2.Pipe[IO, App, WorkerParams] =
    _.evalMap {
      case app @ App(appId, storageHash, _) =>
        println(s"we're in prepareWorkerParams!")
        println(
          s"validatorKey: ${Web3jConverters.bytes32ToHexString(workerId)}"
        )
        for {
          _ ← IO { logger.info("This node will host app '{}'", appId) }

          tmDir ← IO(rootPath.resolve("tendermint"))
          templateConfigDir ← IO(tmDir.resolve("config"))
          workerPath ← IO(tmDir.resolve(s"${appId}_{$app.cluster.currentWorker.index}"))
          workerConfigDir ← IO(workerPath.resolve("config"))

          _ ← IO { Files.createDirectories(workerConfigDir) }

          _ ← TendermintConfig.copyMasterKeys(templateConfigDir, workerConfigDir)
          _ ← TendermintConfig.writeGenesis(app, workerConfigDir)
          _ ← TendermintConfig.updateConfigTOML(
            app,
            workerId,
            configSrc = templateConfigDir.resolve("default_config.toml"),
            configDest = workerConfigDir.resolve("config.toml")
          )

          codePath ← codeManager.prepareCode(CodePath(storageHash), workerPath)
        } yield
          WorkerParams(
            app.appIdHex,
            app.cluster.currentWorker,
            workerPath.toString,
            codePath,
            masterNodeContainerId,
            workerImage
          )
    }

  def writeGenesis(app: App, dest: Path) = IO {
    import io.circe.syntax._

    val df = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'")
    df.setTimeZone(TimeZone.getTimeZone("UTC"))

    val genesis = Map(
      "genesis_time" -> df.format(app.cluster.genesisTime.toMillis),
      "chain_id" -> Web3jConverters.bytes32AppIdToChainId(app.appId),
      "app_hash" -> "",
      "validators" -> app.cluster.workers.map { w =>
        Map(
          "pub_key" -> Map(
            "type" -> "tendermint/PubKeyEd25519",
            "value" -> w.base64ValidatorKey
          ).asJson.spaces2,
          "power" -> "1",
          "name" -> s"${app.appId}_${w.index}"
        )
      }.asJson.spaces2
    )

    logger.info("Writing {}/genesis.json", dest)
    Files.write(dest.resolve("genesis.json"), genesis.asJson.spaces2.getBytes)
  }

  def updateConfigTOML(app: App, workerId: Bytes32, configSrc: Path, configDest: Path) = IO {
    import scala.collection.JavaConverters._
    logger.info("Updating {} -> {}", configSrc, configDest)

    val currentWorker = app.cluster.currentWorker
    val externalAddress = currentWorker.peerAddress

    val persistentPeers = app.cluster.workers.map(_.peerAddress).mkString(",")

    val lines = Source.fromFile(configSrc.toUri).getLines().map {
      case s if s.contains("external_address") => s"""external_address = "$externalAddress""""
      case s if s.contains("persistent_peers") => s"""persistent_peers = "$persistentPeers""""
      case s if s.contains("moniker") => s"""moniker = "${app.appId}_${currentWorker.index}""""
      case s => s
    }

    Files.write(configDest, lines.toIterable.asJava)
  }

  def copyMasterKeys(from: Path, to: Path): IO[Path] = {
    import StandardCopyOption.REPLACE_EXISTING

    val nodeKey = "node_key.json"
    val validator = "priv_validator.json"

    IO {
      logger.info(s"Copying keys to worker: ${from.resolve(nodeKey)} -> ${to.resolve(nodeKey)}")
      Files.copy(from.resolve(nodeKey), to.resolve(nodeKey), REPLACE_EXISTING)

      logger.info(s"Copying priv_validator to worker: ${from.resolve(validator)} -> ${to.resolve(validator)}")
      Files.copy(from.resolve(validator), to.resolve(validator), REPLACE_EXISTING)
    }
  }
}
