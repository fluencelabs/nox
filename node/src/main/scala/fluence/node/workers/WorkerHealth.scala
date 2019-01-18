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

import fluence.node.eth.WorkerNode
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
  lastBlock: String,
  lastAppHash: String,
  lastBlockHeight: Int
)

object RunningWorkerInfo {

  def fromParams(worker: WorkerNode, tendermintInfo: WorkerTendermintInfo) =
    RunningWorkerInfo(
      worker.rpcPort,
      worker.port,
      worker.smPrometheusPort,
      worker.tmPrometheusPort,
      tendermintInfo.node_info.id,
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
)

object StoppedWorkerInfo {

  def apply(
    worker: WorkerNode
  ): StoppedWorkerInfo =
    new StoppedWorkerInfo(
      worker.rpcPort,
      worker.port,
      worker.smPrometheusPort,
      worker.tmPrometheusPort,
    )

  implicit val encodeStoppedWorkerInfo: Encoder[StoppedWorkerInfo] = deriveEncoder
  implicit val decodeStoppedWorkerInfo: Decoder[StoppedWorkerInfo] = deriveDecoder
}

sealed trait WorkerHealth {
  def isHealthy: Boolean
}

object WorkerHealth {
  implicit val encodeThrowable: Encoder[Throwable] = Encoder[String].contramap(_.getLocalizedMessage)

  implicit val decodeThrowable: Decoder[Throwable] = Decoder[String].map(s => new Exception(s))

  implicit val encoderWorkerInfo: Encoder[WorkerHealth] = deriveEncoder
  implicit val decoderWorkerInfo: Decoder[WorkerHealth] = deriveDecoder
}

sealed trait WorkerHealthy extends WorkerHealth {
  override def isHealthy: Boolean = true
}

sealed trait WorkerIll extends WorkerHealth {
  override def isHealthy: Boolean = false
}

case class WorkerRunning(uptime: Long, info: RunningWorkerInfo) extends WorkerHealthy

case class WorkerNotYetLaunched(info: StoppedWorkerInfo) extends WorkerIll

case class WorkerContainerNotRunning(info: StoppedWorkerInfo) extends WorkerIll

case class WorkerHttpCheckFailed(info: StoppedWorkerInfo, causedBy: Throwable) extends WorkerIll
