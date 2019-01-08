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

package fluence.node.workers

import fluence.node.workers.WorkerResponse.WorkerTendermintInfo
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto._

/**
 * Collected information about running worker.
 */
case class RunningWorkerInfo(
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

object RunningWorkerInfo {

  def fromParams(params: WorkerParams, tendermintInfo: WorkerTendermintInfo) =
    RunningWorkerInfo(
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

  implicit val encodeRunningWorkerInfo: Encoder[RunningWorkerInfo] = deriveEncoder
  implicit val decodeRunningWorkerInfo: Decoder[RunningWorkerInfo] = deriveDecoder
}

/**
 * Collected information about a stopped worker.
 */
case class StoppedWorkerInfo(
  rpcPort: Short,
  p2pPort: Short,
  stateMachinePrometheusPort: Short,
  tendermintPrometheusPort: Short,
  codeId: String
)

object StoppedWorkerInfo {

  def apply(
    params: WorkerParams
  ): StoppedWorkerInfo =
    new StoppedWorkerInfo(
      params.clusterData.rpcPort,
      params.clusterData.p2pPort,
      params.clusterData.smPrometheusPort,
      params.clusterData.tmPrometheusPort,
      params.clusterData.code.asHex
    )

  implicit val encodeStoppedWorkerInfo: Encoder[StoppedWorkerInfo] = deriveEncoder
  implicit val decodeStoppedWorkerInfo: Decoder[StoppedWorkerInfo] = deriveDecoder
}

sealed trait WorkerInfo {
  def isHealthy: Boolean
}

object WorkerInfo {
  implicit val encodeThrowable: Encoder[Throwable] = Encoder[String].contramap(_.getLocalizedMessage)

  implicit val decodeThrowable: Decoder[Throwable] = Decoder[String].map(s => new Exception(s))

  implicit val encoderWorkerInfo: Encoder[WorkerInfo] = deriveEncoder
  implicit val decoderWorkerInfo: Decoder[WorkerInfo] = deriveDecoder
}

sealed trait WorkerHealthy extends WorkerInfo {
  override def isHealthy: Boolean = true
}

sealed trait WorkerIll extends WorkerInfo {
  override def isHealthy: Boolean = false
}

case class WorkerRunning(uptime: Long, info: RunningWorkerInfo) extends WorkerHealthy

case class WorkerNotYetLaunched(info: StoppedWorkerInfo) extends WorkerIll

case class WorkerContainerNotRunning(info: StoppedWorkerInfo) extends WorkerIll

case class WorkerHttpCheckFailed(info: StoppedWorkerInfo, causedBy: Throwable) extends WorkerIll
