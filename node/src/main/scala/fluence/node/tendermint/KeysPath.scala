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
import java.nio.file.StandardCopyOption.REPLACE_EXISTING
import java.nio.file.{Files, Path, Paths}

import cats.effect.IO
import fluence.node.docker.DockerParams
import io.circe.parser.parse

/**
 * Wraps tendermint directory for the Master process (contains shared tendermint keys).
 *
 * @param masterTendermintPath Tendermint's home directory
 */
case class KeysPath(masterTendermintPath: String) {

  /**
   * Runs `tendermint show_validator` inside the solver's container, and returns its output as [[ValidatorKey]].
   */
  val showValidatorKey: IO[ValidatorKey] =
    for {
      validatorKeyStr ← solverExec("tendermint show_validator --home=\"/tendermint\"")

      validatorKey ← IO.fromEither(
        parse(validatorKeyStr).flatMap(_.as[ValidatorKey])
      )
    } yield validatorKey

  /**
   * Runs `tendermint show_node_id` inside the solver's container, and returns its output.
   */
  val showNodeId: IO[String] =
    solverExec("tendermint show_node_id --home=\"/tendermint\"")

  /**
   * Executes a command inside solver's container, binding tendermint's home directory into `/tendermint` volume.
   *
   * @param command The command to execute
   */
  private def solverExec(command: String): IO[String] =
    IO(
      DockerParams
        .exec()
        .volume(masterTendermintPath, "/tendermint")
        // TODO: it could be another image, specific to tendermint process only, no need to take solver
        .image("fluencelabs/solver:latest")
        .exec(command)
        .!!
    )

  lazy val path: Path = Paths.get(masterTendermintPath)

  /**
   * Copies master tendermint keys to solver path
   *
   * @param solverTendermintPath Solver's tendermint path
   */
  def copyKeysToSolver(solverTendermintPath: Path): IO[Unit] = IO {
    Files.copy(
      path.resolve("config").resolve("node_key.json"),
      solverTendermintPath.resolve("config").resolve("node_key.json"),
      REPLACE_EXISTING
    )

    Files.copy(
      path.resolve("config").resolve("priv_validator.json"),
      solverTendermintPath.resolve("config").resolve("priv_validator.json"),
      REPLACE_EXISTING
    )
  }
}
