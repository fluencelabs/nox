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

package fluence.node.workers.health
import fluence.node.workers.{DockerWorker, WorkerParams}
import fluence.node.workers.tendermint.status.StatusResponse.WorkerTendermintInfo
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}

/**
 * Collected information about running worker.
 */
case class RunningWorkerInfo(
  appId: Long,
  rpcPort: Short,
  p2pPort: Short,
  stateMachinePrometheusPort: Short,
  tendermintPrometheusPort: Short,
  tendermintNodeId: String,
  lastBlock: String,
  lastAppHash: String,
  lastBlockHeight: Int
)

object RunningWorkerInfo {

  def apply(params: WorkerParams, tendermintInfo: WorkerTendermintInfo): RunningWorkerInfo =
    RunningWorkerInfo(
      params.appId,
      params.currentWorker.rpcPort,
      params.currentWorker.p2pPort,
      DockerWorker.SM_PROMETHEUS_PORT,
      DockerWorker.TM_PROMETHEUS_PORT,
      tendermintInfo.node_info.id,
      tendermintInfo.sync_info.latest_block_hash,
      tendermintInfo.sync_info.latest_app_hash,
      tendermintInfo.sync_info.latest_block_height
    )

  implicit val encodeRunningWorkerInfo: Encoder[RunningWorkerInfo] = deriveEncoder
  implicit val decodeRunningWorkerInfo: Decoder[RunningWorkerInfo] = deriveDecoder
}
