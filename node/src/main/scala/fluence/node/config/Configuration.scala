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

package fluence.node.config

import java.nio.file.{Path, Paths}

import cats.effect.{ContextShift, IO, Sync}
import com.typesafe.config.Config
import fluence.node.config.ConfigOps._
import fluence.node.docker.DockerImage
import fluence.node.workers.tendermint.{DockerTendermint, ValidatorKey}
import io.circe.parser._

import scala.language.higherKinds

// TODO this is the configuration for what? why so many fields are taken from MasterConfig? could we simplify?
case class Configuration(
  rootPath: Path,
  nodeConfig: NodeConfig
)

object Configuration extends slogging.LazyLogging {

  /**
   * Load config at /master/application.conf with fallback on config from class loader
   */
  def loadConfig(): IO[Config] = {
    import pureconfig.backend.ConfigFactoryWrapper._
    val containerConfig = "/master/application.conf"

    (loadFile(Paths.get(containerConfig)) match {
      case Left(_) => load() // exception will be printed out later, see ConfigOps
      case Right(config) => load.map(config.withFallback)
    }).toIO
  }

  // TODO avoid this! it's not configuration, and what is being done there is very obscure!
  def init(masterConfig: MasterConfig)(implicit ec: ContextShift[IO]): IO[Configuration] =
    for {
      rootPath <- IO(Paths.get(masterConfig.rootPath).toAbsolutePath)
      t <- tendermintInit(masterConfig.masterContainerId, rootPath, masterConfig.tendermintImage)
      (nodeId, validatorKey) = t
      nodeConfig = NodeConfig(
        masterConfig.endpoints,
        validatorKey,
        nodeId,
        masterConfig.workerImage,
        masterConfig.tendermintImage
      )
    } yield
      Configuration(
        rootPath,
        nodeConfig
      )

  /**
   * Run `tendermint --init` in container to initialize /master/tendermint/config with configuration files.
   * Later, files /master/tendermint/config are used to run and configure workers
   * TODO move it to DockerTendermint?
   *
   * @param masterContainerId id of master docker container (container running this code), if it's run inside Docker
   * @param rootPath MasterNode's root path
   * @param tmImage Docker image for Tendermint, used to run Tendermint that is bundled inside
   * @return nodeId and validator key
   */
  private def tendermintInit(masterContainerId: Option[String], rootPath: Path, tmImage: DockerImage)(
    implicit c: ContextShift[IO]
  ): IO[(String, ValidatorKey)] = {

    val tendermintDir = rootPath.resolve("tendermint") // /master/tendermint
    def execTendermintCmd[F[_]: Sync: ContextShift](cmd: String, uid: String): F[String] =
      DockerTendermint.execCmd[F](tmImage, tendermintDir, masterContainerId, cmd, uid)

    for {
      uid <- IO(scala.sys.process.Process("id -u").!!.trim)
      //TODO: don't do tendermint init if keys already exist
      _ <- execTendermintCmd[IO]("init", uid)

      _ <- IO {
        tendermintDir.resolve("config").resolve("genesis.json").toFile.delete()
        tendermintDir.resolve("data").toFile.delete()
      }

      nodeId <- execTendermintCmd[IO]("show_node_id", uid)
      _ <- IO { logger.info(s"Node ID: $nodeId") }

      validatorRaw <- execTendermintCmd[IO]("show_validator", uid)
      validator <- IO.fromEither(parse(validatorRaw).flatMap(_.as[ValidatorKey]))
      _ <- IO { logger.info(s"Validator PubKey: ${validator.value}") }
    } yield (nodeId, validator)
  }
}
