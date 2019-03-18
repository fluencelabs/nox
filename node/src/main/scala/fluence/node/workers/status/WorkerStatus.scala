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

package fluence.node.workers.status
import fluence.effects.tendermint.rpc.response.TendermintStatus
import io.circe.generic.semiauto._
import io.circe.{Decoder, Encoder}

/**
 * Worker status representation
 *
 * @param isHealthy True iff both Tendermint and Statemachine operate correctly
 * @param appId App id
 * @param tendermint Tendermint status
 * @param worker Worker status
 */
case class WorkerStatus(
  isHealthy: Boolean,
  appId: Long,
  tendermint: ServiceStatus[TendermintStatus],
  worker: ServiceStatus[Unit],
)

object WorkerStatus {
  implicit val encoderWorkerInfo: Encoder[WorkerStatus] = deriveEncoder
  implicit val decoderWorkerInfo: Decoder[WorkerStatus] = deriveDecoder

}
