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
import fluence.node.tendermint.{KeysPath, ValidatorKey}

import scala.util.Try

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
  private val PortRangeLengthLimits = 1 to 100
  private val StartPortLimits = 20000 to (30000 - PortRangeLengthLimits.max)

  /**
   * Builds [[NodeConfig]] from command-line arguments.
   *
   * @param args arguments list
   * - Tendermint p2p host IP
   * - Tendermint p2p port range starting port
   * - Tendermint p2p port range ending port
   */
  def fromArgs(keysPath: KeysPath, args: List[String]): IO[NodeConfig] =
    for {
      argsTuple ← args match {
        case a1 :: a2 :: a3 :: Nil ⇒ IO.pure(a1, a2, a3)
        case _ ⇒ IO.raiseError(new IllegalArgumentException("4 program arguments expected"))
      }

      (ip, startPortString, endPortString) = argsTuple

      validatorKey ← keysPath.showValidatorKey
      nodeAddress ← keysPath.showNodeId

      _ = logger.info("Tendermint node id: {}", nodeAddress)

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
    } yield NodeConfig(ip, startPort, endPort, validatorKey, nodeAddress)

}
