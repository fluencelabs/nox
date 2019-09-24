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

package fluence.worker.api

import fluence.bp.api.BlockProducerStatus
import fluence.effects.EffectError
import fluence.statemachine.api.data.StateMachineStatus

sealed abstract class WorkerStatus(val isOperating: Boolean)

case class WorkerFailing(
  machine: Either[EffectError, StateMachineStatus],
  producer: Either[EffectError, BlockProducerStatus]
) extends WorkerStatus(false)

case class WorkerOperating(machine: StateMachineStatus, producer: BlockProducerStatus) extends WorkerStatus(true)
