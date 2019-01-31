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
import java.nio.file.{Path, Paths}

import cats.effect.{ContextShift, IO, Sync}
import fluence.node.config.{MasterConfig, NodeConfig, StatusServerConfig, SwarmConfig}
import ConfigOps._
import com.typesafe.config.Config
import fluence.node.docker.{DockerIO, DockerImage, DockerParams}
import fluence.node.eth.conf.{EthereumRpcConfig, FluenceContractConfig}
import fluence.node.workers.tendermint.ValidatorKey
import io.circe.parser._
import pureconfig.generic.auto._

import scala.language.higherKinds

// TODO this is the configuration for what? why so many fields are taken from MasterConfig? could we simplify?
case class Configuration(
  rootPath: Path,
  nodeConfig: NodeConfig,
  contractConfig: FluenceContractConfig,
  swarmConfig: Option[SwarmConfig],
  statsServerConfig: StatusServerConfig,
  ethereumRpcConfig: EthereumRpcConfig,
  masterContainerId: Option[String]
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

  def create()(implicit ec: ContextShift[IO]): IO[(MasterConfig, Configuration)] =
    for {
      config <- loadConfig()
      masterConfig <- pureconfig.loadConfig[MasterConfig](config).toIO
      rootPath <- IO(Paths.get(masterConfig.tendermintPath).toAbsolutePath)
      t <- tendermintInit(masterConfig.masterContainerId, rootPath, masterConfig.worker)
      (nodeId, validatorKey) = t
      nodeConfig = NodeConfig(masterConfig.endpoints, validatorKey, nodeId, masterConfig.worker)
    } yield
      (
        masterConfig,
        Configuration(
          rootPath,
          nodeConfig,
          masterConfig.contract,
          masterConfig.swarm,
          masterConfig.statusServer,
          masterConfig.ethereum,
          masterConfig.masterContainerId
        )
      )

  /**
   * Run `tendermint --init` in container to initialize /master/tendermint/config with configuration files.
   * Later, files /master/tendermint/config are used to run and configure workers
   *
   * @param masterContainerId id of master docker container (container running this code), if it's run inside Docker
   * @return nodeId and validator key
   */
  def tendermintInit(masterContainerId: Option[String], rootPath: Path, workerImage: DockerImage)(
    implicit c: ContextShift[IO]
  ): IO[(String, ValidatorKey)] = {

    val tendermintDir = rootPath.resolve("tendermint") // /master/tendermint
    def tendermint[F[_]: Sync: ContextShift](cmd: String, uid: String): F[String] =
      DockerIO.exec[F](
        masterContainerId
          .foldLeft(
            DockerParams
              .build()
              .user(uid)
          )(_.option("--volumes-from", _))
          .image(workerImage)
          .run("tendermint", cmd, s"--home=$tendermintDir")
      )

    for {
      uid <- IO(scala.sys.process.Process("id -u").!!.trim)
      //TODO: don't do tendermint init if keys already exist
      _ <- tendermint[IO]("init", uid)

      _ <- IO {
        tendermintDir.resolve("config").resolve("config.toml").toFile.delete()
        tendermintDir.resolve("config").resolve("genesis.json").toFile.delete()
        tendermintDir.resolve("data").toFile.delete()
      }

      nodeId <- tendermint[IO]("show_node_id", uid)
      _ <- IO { logger.info(s"Node ID: $nodeId") }

      validatorRaw <- tendermint[IO]("show_validator", uid)
      validator <- IO.fromEither(parse(validatorRaw).flatMap(_.as[ValidatorKey]))
      _ <- IO { logger.info(s"Validator PubKey: ${validator.value}") }
    } yield (nodeId, validator)
  }
}
