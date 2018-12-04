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

import cats.effect.{ContextShift, IO, Sync}
import cats.syntax.applicativeError._
import cats.syntax.flatMap._
import fluence.node.solvers.SolverImage
import fluence.node.tendermint.ValidatorKey

import scala.language.higherKinds

/**
 * Information about a node possible endpoints (IP and ports) that will be used as addresses
 * for requests after a cluster will be formed
 *
 * @param ip p2p host IP
 * @param minPort starting port for p2p port range
 * @param maxPort ending port for p2p port range
**/
case class EndpointsConfig(
  ip: InetAddress,
  minPort: Short,
  maxPort: Short
)

/**
 * Information about a node willing to run solvers to join Fluence clusters.
 *
 * @param endpoints information about a node possible endpoints (IP and ports) that will be used as addresses
 *                 for requests after a cluster will be formed
 * @param validatorKey p2p port
 * @param nodeAddress p2p port
 */
case class NodeConfig(
  endpoints: EndpointsConfig,
  validatorKey: ValidatorKey,
  nodeAddress: String,
  solverImage: SolverImage
)
