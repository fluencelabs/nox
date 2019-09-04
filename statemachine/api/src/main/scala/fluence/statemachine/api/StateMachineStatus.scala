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

package fluence.statemachine.api

import io.circe.{Decoder, Encoder}
import io.circe.generic.semiauto._

/**
 * Statemachine's status, fetched via Control RPC.
 *
 * @param expectsEth Whether this statemachine expects to get Ethereum blocks or not
 * @param stateHash Actual Statemachine's state hash
 */
case class StateMachineStatus(
  expectsEth: Boolean,
  stateHash: StateHash
)

object StateMachineStatus {
  implicit val controlStatusEncoder: Encoder[StateMachineStatus] = deriveEncoder[StateMachineStatus]
  implicit val controlStatusDecoder: Decoder[StateMachineStatus] = deriveDecoder[StateMachineStatus]
}
