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

package fluence.effects.tendermint.rpc

import fluence.effects.{EffectError, WithCause}

sealed trait WRpcError extends EffectError

private[rpc] case object Disconnected extends WRpcError {
  override def getMessage: String = "websocket received close early"
}

private[rpc] case class DisconnectedWithError(cause: Throwable) extends WRpcError with WithCause[Throwable] {
  override def getMessage: String = s"websocket closed due to error: $cause"
}
private[rpc] case class ConnectionFailed(cause: Throwable) extends WRpcError with WithCause[Throwable] {
  override def getMessage: String = s"connection failed: $cause"
}
