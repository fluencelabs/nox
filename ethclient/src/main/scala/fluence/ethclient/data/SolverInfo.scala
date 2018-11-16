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

package fluence.ethclient.data

import java.io.File

import cats.data.EitherT
import cats.effect.IO
import fluence.ethclient.helpers.DockerRunBuilder
import fluence.ethclient.helpers.Web3jConverters.{base64ToBytes32, solverAddressToBytes24}
import io.circe.generic.auto._
import io.circe.parser.parse
import org.web3j.abi.datatypes.generated.{Bytes24, Bytes32, Uint16}

import scala.language.postfixOps
import scala.sys.process._
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
case class SolverInfo(
  longTermLocation: String,
  ip: String,
  startPort: Short,
  endPort: Short,
  validatorKey: TendermintValidatorKey,
  nodeAddress: String
) {

  /**
   * Returns node's public key in format ready to pass to the contract.
   */
  lazy val validatorKeyBytes32: Bytes32 = base64ToBytes32(validatorKey.value)

  /**
   * Returns node's address information (host, Tendermint p2p key) in format ready to pass to the contract.
   */
  lazy val addressBytes24: Bytes24 = solverAddressToBytes24(ip, nodeAddress)

  /**
   * Returns starting port as uint16.
   */
  lazy val startPortUint16: Uint16 = new Uint16(startPort)

  /**
   * Returns ending port as uint16.
   */
  lazy val endPortUint16: Uint16 = new Uint16(endPort)
}

object SolverInfo {

  private val portRangeLengthLimits = 1 to 100
  private val startPortLimits = 20000 to (30000 - portRangeLengthLimits.max)

  /**
   * Builds [[SolverInfo]] from command-line arguments.
   *
   * @param args arguments list
   * - Tendermint key location
   * - Tendermint p2p host IP
   * - Tendermint p2p port range starting port
   * - Tendermint p2p port range ending port
   */
  def apply(args: List[String]): EitherT[IO, Throwable, SolverInfo] =
    for {
      argsTuple <- args match {
        case List(a1, a2, a3, a4) => EitherT.right(IO.pure(a1, a2, a3, a4))
        case _ => EitherT.left(IO.pure(new IllegalArgumentException("4 program arguments expected")))
      }
      (longTermLocation, ip, startPortString, endPortString) = argsTuple

      dockerWorkDir = System.getProperty("user.dir") + "/statemachine/docker"

      validatorKeyStr <- EitherT.fromEither[IO](
        Try(
          Process(
            DockerRunBuilder()
              .addVolume(dockerWorkDir + "/tm-show-validator", "/solver")
              .addVolume(longTermLocation, "/tendermint")
              .build("fluencelabs/solver:latest"),
            new File(dockerWorkDir)
          ).!!
        ).toEither
      )
      validatorKey <- EitherT.fromEither[IO](parse(validatorKeyStr).flatMap(_.as[TendermintValidatorKey]))

      nodeAddress <- EitherT.fromEither[IO](
        Try(
          Process(
            DockerRunBuilder()
              .addVolume(dockerWorkDir + "/tm-show-node-id", "/solver")
              .addVolume(longTermLocation, "/tendermint")
              .build("fluencelabs/solver:latest"),
            new File(dockerWorkDir)
          ).!!
        ).toEither
      )

      _ <- EitherT.cond[IO][Throwable, Unit](isValidIP(ip), (), new IllegalArgumentException(s"Incorrect IP: $ip"))

      startPort <- EitherT.fromEither[IO](Try(startPortString.toShort).toEither)
      endPort <- EitherT.fromEither[IO](Try(endPortString.toShort).toEither)

      _ <- EitherT.cond[IO][Throwable, Unit](
        portRangeLengthLimits.contains(endPort - startPort) && startPortLimits.contains(startPort),
        (),
        new IllegalArgumentException(
          s"Port range should contain $portRangeLengthLimits ports starting from $startPortLimits"
        )
      )
    } yield SolverInfo(longTermLocation, ip, startPort, endPort, validatorKey, nodeAddress)

  private def isValidIP(ip: String): Boolean = {
    val parts = ip.split('.')
    parts.length == 4 && parts.forall(x => Try(x.toShort).filter(Range(0, 256).contains(_)).isSuccess)
  }

}
