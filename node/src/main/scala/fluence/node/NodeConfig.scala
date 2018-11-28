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

import java.net.InetAddress

import cats.effect.IO
import fluence.node.tendermint.{KeysPath, ValidatorKey}

import scala.language.higherKinds

/**
 * Information about a node willing to run solvers to join Fluence clusters.
 *
 * @param ip p2p host IP
 * @param startPort starting port for p2p port range
 * @param endPort ending port for p2p port range
 * @param validatorKey p2p port
 * @param nodeAddress p2p port
 */
case class NodeConfig(
  ip: String,
  startPort: Short,
  endPort: Short,
  validatorKey: ValidatorKey,
  nodeAddress: String
)

object NodeConfig extends slogging.LazyLogging {
  private val MaxPortCount = 100
  private val MinPortCount = 0
  private val MinPort = 20000
  private val MaxPort = 40000
  private def MaxPort(range: Int = 0): Int = MaxPort - range

  /**
   * Builds [[NodeConfig]] from command-line arguments.
   *
   * @param args arguments list
   * - Tendermint p2p host IP
   * - Tendermint p2p port range starting port
   * - Tendermint p2p port range ending port
   */
  def fromArgs(keysPath: KeysPath, args: List[String]): IO[NodeConfig] = {
    for {
      argsTuple ← args match {
        case a1 :: a2 :: a3 :: Nil ⇒ IO.pure((a1, a2, a3))
        case _ ⇒ IO.raiseError(new IllegalArgumentException("4 program arguments expected"))
      }

      (ip, startPortString, endPortString) = argsTuple

      validatorKey ← keysPath.showValidatorKey
      nodeAddress ← keysPath.showNodeId

      _ = logger.info("Tendermint node id: {}", nodeAddress.trim)

      _ <- checkIp(ip)

      startPort <- IO(startPortString.toShort)
      endPort <- IO(endPortString.toShort)
      _ ← checkPorts(startPort, endPort)
    } yield NodeConfig(ip, startPort, endPort, validatorKey, nodeAddress)
  }

  private def checkIp(ip: String): IO[Unit] =
    IO(InetAddress.getByName(ip)).attempt.flatMap {
      case Right(_) => IO.unit
      case Left(e) => IO.raiseError(new IllegalArgumentException(s"Incorrect IP: $ip.").initCause(e))
    }

  private def checkPorts(startPort: Int, endPort: Int): IO[Unit] = {
    val ports = endPort - startPort

    if (ports <= MinPortCount || ports > MaxPortCount) {
      IO.raiseError(
        new IllegalArgumentException(
          s"Port range size should be between $MinPortCount and $MaxPortCount"
        )
      )
    } else if (startPort < MinPort || startPort > MaxPort(ports) && endPort > MaxPort) {
      IO.raiseError(
        new IllegalArgumentException(
          s"Allowed ports should be between $MinPort and $MaxPort"
        )
      )
    } else {
      IO.unit
    }
  }

}
