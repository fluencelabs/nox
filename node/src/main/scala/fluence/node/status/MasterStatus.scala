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

package fluence.node.status
import fluence.node.config.{MasterConfig, NodeConfig}
import fluence.node.workers.health.WorkerHealth
import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto._

/**
 * Master node status.
 *
 * @param ip master node ip address
 * @param listOfPorts all available ports to use by code developers
 * @param uptime working time of master node
 * @param numberOfWorkers number of registered workers
 * @param workers info about workers
 * @param config config file
 */
case class MasterStatus(
  ip: String,
  listOfPorts: String,
  uptime: Long,
  nodeConfig: NodeConfig,
  numberOfWorkers: Int,
  workers: List[WorkerHealth],
  config: MasterConfig
)

object MasterStatus {
  implicit val encodeMasterState: Encoder[MasterStatus] = deriveEncoder
  implicit val decodeMasterState: Decoder[MasterStatus] = deriveDecoder
}
