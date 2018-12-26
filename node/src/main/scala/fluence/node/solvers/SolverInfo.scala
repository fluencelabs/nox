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

import fluence.node.solvers.SolverResponse.SolverTendermintInfo
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto._

/**
 * Collected information about running solver.
 */
case class RunningSolverInfo(
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

object RunningSolverInfo {

  def fromParams(params: SolverParams, tendermintInfo: SolverTendermintInfo) =
    RunningSolverInfo(
      params.clusterData.rpcPort,
      params.clusterData.p2pPort,
      params.clusterData.smPrometheusPort,
      params.clusterData.tmPrometheusPort,
      tendermintInfo.node_info.id,
      params.clusterData.code.asHex,
      tendermintInfo.sync_info.latest_block_hash,
      tendermintInfo.sync_info.latest_app_hash,
      tendermintInfo.sync_info.latest_block_height
    )

  implicit val encodeSolverInfo: Encoder[RunningSolverInfo] = deriveEncoder
  implicit val decodeSolverInfo: Decoder[RunningSolverInfo] = deriveDecoder
}

/**
 * Collected information about stopped solver.
 */
case class StoppedSolverInfo(
  rpcPort: Short,
  p2pPort: Short,
  stateMachinePrometheusPort: Short,
  tendermintPrometheusPort: Short,
  codeId: String
)

object StoppedSolverInfo {

  def apply(
    params: SolverParams
  ): StoppedSolverInfo =
    new StoppedSolverInfo(
      params.clusterData.rpcPort,
      params.clusterData.p2pPort,
      params.clusterData.smPrometheusPort,
      params.clusterData.tmPrometheusPort,
      params.clusterData.code.asHex
    )

  implicit val encodeSolverInfo: Encoder[StoppedSolverInfo] = deriveEncoder
  implicit val decodeSolverInfo: Decoder[StoppedSolverInfo] = deriveDecoder
}

sealed trait SolverInfo {
  def isHealthy: Boolean
}

object SolverInfo {
  implicit val encodeThrowable: Encoder[Throwable] = Encoder[String].contramap(_.getLocalizedMessage)

  implicit val decodeThrowable: Decoder[Throwable] = Decoder[String].map(s => new Exception(s))

  implicit val encoderSolverHealth: Encoder[SolverInfo] = deriveEncoder
  implicit val decoderSolverHealth: Decoder[SolverInfo] = deriveDecoder
}

sealed trait SolverHealthy extends SolverInfo {
  override def isHealthy: Boolean = true
}

sealed trait SolverIll extends SolverInfo {
  override def isHealthy: Boolean = false
}

case class SolverRunning(uptime: Long, info: RunningSolverInfo) extends SolverHealthy

case class SolverNotYetLaunched(info: StoppedSolverInfo) extends SolverIll

case class SolverContainerNotRunning(info: StoppedSolverInfo) extends SolverIll

case class SolverHttpCheckFailed(info: StoppedSolverInfo, causedBy: Throwable) extends SolverIll
