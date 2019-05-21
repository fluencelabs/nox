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

package fluence.node.workers.control

import fluence.effects.tendermint.block.history.Receipt
import scodec.bits.ByteVector

import scala.util.control.NoStackTrace

trait ControlRpcError extends NoStackTrace

trait WithCause[E <: Throwable] extends ControlRpcError {
  def cause: E

  initCause(cause)
}

case class DropPeerError(key: ByteVector, cause: Throwable) extends WithCause[Throwable] {
  override def getMessage: String = s"Error dropping peer ${key.toHex}"
}
case class WorkerStatusError(cause: Throwable) extends WithCause[Throwable] {
  override def getMessage: String = "Error retrieving worker status"
}
case class StopError(cause: Throwable) extends WithCause[Throwable] {
  override def getMessage: String = "Error while signaling worker to stop"
}
case class SendBlockReceiptError(receipt: Receipt, cause: Throwable) extends WithCause[Throwable] {
  override def getMessage: String = s"Error sending block receipt ${receipt.hash.toHex}"
}
case class GetVmHashError(cause: Throwable) extends WithCause[Throwable] {
  override def getMessage: String = "Error getting VM hash from worker"
}
