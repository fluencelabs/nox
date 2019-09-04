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

package fluence.statemachine.client

import fluence.effects.{EffectError, WithCause}
import fluence.statemachine.api.signals.BlockReceipt
import scodec.bits.ByteVector

trait ControlRpcError extends EffectError

case class DropPeerError(key: ByteVector, cause: Throwable) extends ControlRpcError with WithCause[Throwable] {
  override def getMessage: String = s"Error dropping peer ${key.toHex} $cause"
}
case class WorkerStatusError(cause: Throwable) extends ControlRpcError with WithCause[Throwable] {
  override def getMessage: String = s"Error retrieving worker status $cause"
}
case class StopError(cause: Throwable) extends ControlRpcError with WithCause[Throwable] {
  override def getMessage: String = s"Error while signaling worker to stop $cause"
}
case class SendBlockReceiptError(receipt: BlockReceipt, cause: Throwable)
    extends ControlRpcError with WithCause[Throwable] {
  override def getMessage: String = s"Error sending block receipt ${new String(receipt.bytes.toArray)} $cause"
}
case class GetVmHashError(cause: Throwable) extends ControlRpcError with WithCause[Throwable] {
  override def getMessage: String = s"Error getting VM hash from worker: $cause"
}
