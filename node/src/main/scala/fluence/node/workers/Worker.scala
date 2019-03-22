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

import fluence.node.workers.control.ControlRpc
import fluence.node.workers.status.WorkerStatus
import fluence.effects.tendermint.rpc.TendermintRpc

import scala.language.higherKinds

// Algebra for DockerWorker
trait Worker[F[_]] {
  // Tendermint p2p port
  def p2pPort: Short

  // App ID worker handles
  def appId: Long

  // RPC connection to tendermint
  def tendermint: TendermintRpc[F]

  // RPC connection to worker
  def control: ControlRpc[F]

  // Stops the worker when F is evaluated
  // Worker could be restarted afterwards
  def stop: F[Unit]

  // Stops the worker and deallocates all resources used by it
  def remove: F[Unit]

  // Retrieves worker's health
  def status: F[WorkerStatus]

  // Human readable description of the worker
  def description: String
}
