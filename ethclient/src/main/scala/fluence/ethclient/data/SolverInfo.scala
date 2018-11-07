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

import fluence.ethclient.helpers.Web3jConverters.{base64ToBytes32, solverAddressToBytes32}
import io.circe.generic.auto._
import io.circe.parser.parse
import org.web3j.abi.datatypes.generated.Bytes32

import scala.language.postfixOps
import scala.sys.process._
import scala.util.Try

/**
 * Information about a single solver willing to join Fluence clusters.
 *
 * @param longTermLocation local directory with pre-initialized Tendermint public/private keys
 * @param ip p2p host IP
 * @param port p2p port
 * @param validatorKey p2p port
 * @param nodeAddress p2p port
 */
case class SolverInfo(
  longTermLocation: String,
  ip: String,
  port: Short,
  validatorKey: TendermintValidatorKey,
  nodeAddress: String
) {

  /**
   * Returns node's public key in format ready to pass to the contract.
   */
  def validatorKeyBytes32: Bytes32 = base64ToBytes32(validatorKey.value)

  /**
   * Returns node's address information (host, port, Tendermint p2p key) in format ready to pass to the contract.
   */
  def addressBytes32: Bytes32 = solverAddressToBytes32(ip, port, nodeAddress)
}

object SolverInfo {

  def apply(args: List[String]): Either[Throwable, SolverInfo] =
    for {
      argsTuple <- args match {
        case List(a1, a2, a3) => Right(a1, a2, a3)
        case _ => Left(new IllegalArgumentException("3 program argument expected"))
      }
      (longTermLocation, ip, portString) = argsTuple

      validatorKeyStr <- Try(
        s"statemachine/docker/master-run-tm-utility.sh statemachine/docker/tm-show-validator $longTermLocation" !!
      ).toEither
      validatorKey <- parse(validatorKeyStr).flatMap(_.as[TendermintValidatorKey])

      nodeAddress <- Try(
        s"statemachine/docker/master-run-tm-utility.sh statemachine/docker/tm-show-node-id $longTermLocation" !!
      ).toEither

      port <- Try(portString.toShort).toEither
      _ <- Either.cond(isValidIP(ip), (), new IllegalArgumentException(s"Incorrect IP: $ip"))
    } yield SolverInfo(longTermLocation, ip, port, validatorKey, nodeAddress)

  private def isValidIP(ip: String): Boolean = {
    val parts = ip.split('.')
    parts.length == 4 && parts.forall(x => Try(x.toShort).filter(Range(0, 256).contains(_)).isSuccess)
  }

}
