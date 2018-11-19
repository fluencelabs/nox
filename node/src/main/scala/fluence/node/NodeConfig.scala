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

import cats.effect.IO
import fluence.node.docker.DockerParams
import fluence.node.tendermint.ValidatorKey
import io.circe.parser._

import scala.util.Try

/**
 * Information about a node willing to run solvers to join Fluence clusters.
 *
 * @param longTermLocation local directory with pre-initialized Tendermint public/private keys
 * @param ip p2p host IP
 * @param startPort starting port for p2p port range
 * @param endPort ending port for p2p port range
 * @param validatorKey p2p port
 * @param nodeAddress p2p port
 */
case class NodeConfig(
  longTermLocation: String,
  ip: String,
  startPort: Short,
  endPort: Short,
  validatorKey: ValidatorKey,
  nodeAddress: String
)

object NodeConfig {
  private val PortRangeLengthLimits = 1 to 100
  private val StartPortLimits = 20000 to (30000 - PortRangeLengthLimits.max)

  /**
   * Builds [[NodeConfig]] from command-line arguments.
   *
   * @param args arguments list
   * - Tendermint key location
   * - Tendermint p2p host IP
   * - Tendermint p2p port range starting port
   * - Tendermint p2p port range ending port
   */
  def fromArgs(args: List[String]): IO[NodeConfig] =
    for {
      argsTuple ← args match {
        case a1 :: a2 :: a3 :: a4 :: Nil ⇒ IO.pure(a1, a2, a3, a4)
        case _ ⇒ IO.raiseError(new IllegalArgumentException("4 program arguments expected"))
      }

      (longTermLocation, ip, startPortString, endPortString) = argsTuple

      validatorKey ← showValidatorKey(longTermLocation)
      nodeAddress ← showNodeId(longTermLocation)

      _ <- {
        // TODO: is it the best way to check IP? Why not to make RemoteAddr?
        val parts = ip.split('.')
        if (parts.length == 4 && parts.forall(x => Try(x.toShort).filter(Range(0, 256).contains(_)).isSuccess))
          IO.unit
        else
          IO.raiseError(new IllegalArgumentException(s"Incorrect IP: $ip"))
      }

      startPort <- IO(startPortString.toShort)
      endPort <- IO(endPortString.toShort)

      _ ← if (PortRangeLengthLimits.contains(endPort - startPort) && StartPortLimits.contains(startPort))
        IO.unit
      else
        IO.raiseError(
          new IllegalArgumentException(
            s"Port range should contain $PortRangeLengthLimits ports starting from $StartPortLimits"
          )
        )
    } yield NodeConfig(longTermLocation, ip, startPort, endPort, validatorKey, nodeAddress)

  /**
   * Runs `tendermint show_validator` inside the solver's container, and returns its output as [[ValidatorKey]].
   *
   * @param longTermLocation Tendermint's home directory
   */
  private def showValidatorKey(longTermLocation: String): IO[ValidatorKey] =
    for {
      validatorKeyStr ← solverExec(longTermLocation, "tendermint show_validator --home=\"/tendermint\"")

      validatorKey ← IO.fromEither(
        parse(validatorKeyStr).flatMap(_.as[ValidatorKey])
      )
    } yield validatorKey

  /**
   * Runs `tendermint show_node_id` inside the solver's container, and returns its output.
   *
   * @param longTermLocation Tendermint's home directory
   */
  private def showNodeId(longTermLocation: String): IO[String] =
    solverExec(longTermLocation, "tendermint show_node_id --home=\"/tendermint\"")

  /**
   * Executes a command inside solver's container, binding tendermint's home directory into `/tendermint` volume.
   *
   * @param longTermLocation Tendermint's home directory
   * @param command The command to execute
   */
  private def solverExec(longTermLocation: String, command: String): IO[String] =
    IO(
      DockerParams
        .exec()
        .volume(longTermLocation, "/tendermint")
        // TODO: image name should be configurable
        .image("fluencelabs/solver:latest")
        .exec(command)
        .!!
    )

}
