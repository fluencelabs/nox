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
import cats.syntax.apply._
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
      t <- tendermintInit(masterConfig.masterContainerId, rootPath, masterConfig.tendermint)
      (nodeId, validatorKey) = t
      nodeConfig = NodeConfig(
        masterConfig.endpoints,
        validatorKey,
        nodeId,
        masterConfig.worker,
        masterConfig.tendermint
      )
    } yield
      Configuration(
        rootPath,
        nodeConfig
      )

  /**
   * Run `tendermint --init` in container to initialize /master/tendermint with the following configuration files:
   * - config/
   * -- config.toml - main Tendermint config. Will be updated for each app via [[fluence.node.workers.tendermint.config.TendermintConfig]].
   * -- genesis.json - Tendermint blockchain config. Will be deleted and generated for each app via [[fluence.node.workers.tendermint.config.GenesisConfig]].
   * -- node_key.json - p2p key. Copied to app dir as is.
   * -- priv_validator_key.json - validator key. Copied to app dir as is.
   * - data/
   * -- priv_validator_state.json - empty validator state. Copied to app dir as is.
   *
   * Later, files /master/tendermint are used to run and configure workers
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

    def init(uid: String): IO[Unit] =
      for {
        _ <- execTendermintCmd[IO]("init", uid)
        _ <- IO(tendermintDir.resolve("config").resolve("genesis.json").toFile.delete())
      } yield ()

    for {
      // Check that old `priv_validator.json` was migrated to `priv_validator_key.json` or raise an exception
      _ <- checkUpdatedPrivValOrRaise(tendermintDir)

      uid <- IO(scala.sys.process.Process("id -u").!!.trim)

      initialized <- initialized(tendermintDir)
      _ <- if (initialized) {
        IO(logger.info("Node is already initialized"))
      } else init(uid)

      nodeId <- execTendermintCmd[IO]("show_node_id", uid)
      _ <- IO { logger.info(s"Node ID: $nodeId") }

      validatorRaw <- execTendermintCmd[IO]("show_validator", uid)
      validator <- IO.fromEither(parse(validatorRaw).flatMap(_.as[ValidatorKey]))
      _ <- IO { logger.info(s"Validator PubKey: ${validator.value}") }
    } yield (nodeId, validator)
  }

  /**
   * Checks that all or none of config.toml, node_key.json, priv_validator_key.json exist
   * @return true if all files exist, false if none exist, raiseError otherwise
   */
  private def initialized(tendermintDir: Path): IO[Boolean] =
    for {
      files <- IO.pure(List("config.toml", "node_key.json", "priv_validator_key.json"))
      configDir <- IO(tendermintDir.resolve("config"))

      r <- IO {
        files.foldLeft((true, false, Iterable.empty[String])) {
          // All - all files exist, any - any of the files exist, notFound - list of missing files
          case ((all, any, notFound), f) =>
            val exists = configDir.resolve(f).toFile.exists()
            val nf = Some(f).filterNot(_ => exists) ++ notFound

            (all && exists, any || exists, nf)
        }
      }
      (all, any, notFound) = r

      // No files should exist or all files
      _ <- if (any && !all) {
        logger.error(
          s"Unable to execute tendermint init: $configDir is in inconsistent state. " +
            s"Missing files: ${notFound.mkString(", ")}"
        )
        IO.raiseError(new Exception(s"Unable to execute tendermint init: $configDir is in inconsistent state. "))
      } else IO.unit
    } yield any && all

  /**
   * Check that old `priv_validator.json` was migrated to `priv_validator_key.json`
   * See for more details https://github.com/tendermint/tendermint/blob/master/UPGRADING.md#v0280
   */
  private def checkUpdatedPrivValOrRaise(tendermintDir: Path): IO[Unit] =
    for {
      configDir <- IO(tendermintDir.resolve("config"))
      old <- IO(configDir.resolve("priv_validator.json").toFile.exists())
      _ <- if (old) {
        IO(
          logger.info(
            s"priv_validator.json is found in $configDir.\n" +
              s"Since Tendermint 0.28.0 it was replaced by priv_validator_key.json and priv_validator_state.json.\n" +
              "Please, run migration. See https://github.com/tendermint/tendermint/blob/master/UPGRADING.md#v0280\n"
          )
        ) *> IO.raiseError(
          new RuntimeException(
            s"priv_validator.json is found in $configDir. " +
              "See https://github.com/tendermint/tendermint/blob/master/UPGRADING.md#v0280"
          )
        )
      } else IO.unit
    } yield {}
}
