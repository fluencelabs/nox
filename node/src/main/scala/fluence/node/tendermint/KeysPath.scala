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

package fluence.node.tendermint
import java.nio.file.{Files, Path, Paths, StandardCopyOption}

import cats.effect.{ContextShift, IO}
import fluence.node.Configuration
import fluence.node.docker.{DockerIO, DockerParams}
import io.circe.parser.parse

/**
 * Wraps tendermint directory for the Master process (contains shared tendermint keys).
 *
 * @param masterTendermintPath Tendermint's home directory
 */
case class KeysPath(masterTendermintPath: String)(implicit ec: ContextShift[IO]) extends slogging.LazyLogging {

  val path: IO[Path] = IO { Paths.get(masterTendermintPath) } //TODO: convert InvalidPathException to Fluence error

  val nodeKeyPath: IO[Path] = path.map(_.resolve("config").resolve("node_key.json"))
  val privValidatorPath: IO[Path] = path.map(_.resolve("config").resolve("priv_validator.json"))

  /**
   * Runs `tendermint show_validator` inside the solver's container, and returns its output as [[ValidatorKey]].
   */
  def showValidatorKey(implicit ec: ContextShift[IO]): IO[ValidatorKey] =
    for {
      validatorKeyStr ← solverExec("tendermint", "show_validator", "--home=/tendermint")

      validatorKey ← IO.fromEither(
        parse(validatorKeyStr).flatMap(_.as[ValidatorKey])
      )
    } yield validatorKey

  /**
   * Runs `tendermint show_node_id` inside the solver's container, and returns its output.
   */
  def showNodeId(implicit ec: ContextShift[IO]): IO[String] =
    solverExec("tendermint", "show_node_id", "--home=/tendermint")

  /**
   * Executes a command inside solver's container, binding tendermint's home directory into `/tendermint` volume.
   * Container starts anew on every call, with existing tendermint config attached
   *
   * @param executable The command to execute
   */
  private def solverExec(executable: String, params: String*)(implicit ec: ContextShift[IO]): IO[String] =
    for {
      uid <- IO(scala.sys.process.Process("id -u").!!.trim)
      result <- DockerIO
        .run[IO](
          DockerParams
            .run(executable, params: _*)
            .user(uid)
            .volume(masterTendermintPath, "/tendermint")
            // TODO: it could be another image, specific to tendermint process only, no need to take solver
            .image(Configuration.SOLVER_IMAGE)
        )
        .compile
        .lastOrError
    } yield result

  /**
   * Copies master tendermint keys to solver path
   *
   * @param solverTendermintPath Solver's tendermint path
   */
  def copyKeysToSolver(solverTendermintPath: Path): IO[Unit] = path.flatMap { p =>
    IO {
      logger.info(
        s"Copying keys to solver: ${p.resolve("config/node_key.json")} -> " +
          s"${solverTendermintPath.resolve("config/node_key.json")}"
      )
      Files.copy(
        p.resolve("config/node_key.json"),
        solverTendermintPath.resolve("config/node_key.json"),
        StandardCopyOption.REPLACE_EXISTING
      )

      logger.info(
        s"Copying priv_validator to solver: ${p.resolve("config/priv_validator.json")} -> " +
          s"${solverTendermintPath.resolve("config/priv_validator.json")}"
      )
      Files.copy(
        p.resolve("config/priv_validator.json"),
        solverTendermintPath.resolve("config/priv_validator.json"),
        StandardCopyOption.REPLACE_EXISTING
      )
    }
  }
}
