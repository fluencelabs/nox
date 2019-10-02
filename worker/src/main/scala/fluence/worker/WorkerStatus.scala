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

package fluence.worker

import fluence.bp.api.BlockProducerStatus
import fluence.statemachine.api.data.StateMachineStatus
import io.circe.generic.semiauto.{deriveDecoder, deriveEncoder}
import io.circe.{Decoder, Encoder}

/**
 * Complete status of all Worker's components, namely State Machine and Block Producer
 *
 * @param isOperating Whether Worker is operating or not
 */
sealed abstract class WorkerStatus(val isOperating: Boolean)

/**
 * [[Worker]] is not yet allocated, see current [[WorkerStage]]
 *
 * @param stage Current stage
 */
case class WorkerNotAllocated(stage: WorkerStage) extends WorkerStatus(false)

/**
 * One of main [[Worker]]'s components does not respond
 *
 * @param machine Either error or status
 * @param producer Either error or status
 */
case class WorkerFailing(
  machine: Either[String, StateMachineStatus],
  producer: Either[String, BlockProducerStatus]
) extends WorkerStatus(false)

/**
 * Worker is operating, all components launched
 * TODO: check statuses to ensure that components are also OK.
 *
 * @param machine Machine status
 * @param producer Producer status
 */
case class WorkerOperating(
  machine: StateMachineStatus,
  producer: BlockProducerStatus
) extends WorkerStatus(true)

object WorkerStatus {
  private implicit def eitherEncoder[T: Encoder]: Encoder[Either[String, T]] = Encoder.encodeEither("error", "status")
  private implicit def eitherDecoder[T: Decoder]: Decoder[Either[String, T]] = Decoder.decodeEither("error", "status")

  implicit val encoder: Encoder[WorkerStatus] = deriveEncoder
  implicit val decoder: Decoder[WorkerStatus] = deriveDecoder
}
