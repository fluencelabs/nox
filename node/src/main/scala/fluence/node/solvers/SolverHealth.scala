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

package fluence.node.solvers

import io.circe.{Encoder, Json}
import io.circe.generic.semiauto._

/**
  * Collected information about a solver.
  */
case class SolverInfo(
  rpcPort: Short,
  p2pPort: Short,
  stateMachinePrometheusPort: Short,
  tendermintPrometheusPort: Short,
  clusterId: String,
  codeId: String,
  lastBlock: String,
  lastAppHash: String,
  lastBlockHeight: Int
)

object SolverInfo {
  implicit val encodeSolverInfo: Encoder[SolverInfo] = deriveEncoder
}

sealed trait SolverHealth {
  def uptime: Long

  def isHealthy: Boolean
}

object SolverHealth {
  implicit val encodeThrowable: Encoder[Throwable] = new Encoder[Throwable] {
    final def apply(a: Throwable): Json = Json.fromString(a.getLocalizedMessage)
  }

  import SolverInfo._
  implicit val encoderSolverHealth: Encoder[SolverHealth] = deriveEncoder
}

sealed trait SolverHealthy extends SolverHealth {
  override def isHealthy: Boolean = true
}

sealed trait SolverIll extends SolverHealth {
  override def isHealthy: Boolean = false
}

case class SolverRunning(uptime: Long, solverInfo: SolverInfo) extends SolverHealthy

case object SolverNotYetLaunched extends SolverIll {
  override def uptime: Long = 0
}

case class SolverContainerNotRunning(uptime: Long) extends SolverIll

case class SolverHttpCheckFailed(uptime: Long, causedBy: Throwable) extends SolverIll
