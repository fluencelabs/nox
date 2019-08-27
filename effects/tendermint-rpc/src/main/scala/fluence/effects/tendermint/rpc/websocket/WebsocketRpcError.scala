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

package fluence.effects.tendermint.rpc.websocket

import cats.Eval
import fluence.effects.tendermint.block.errors.TendermintBlockError
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.effects.{EffectError, WithCause}
import io.circe.{DecodingFailure, ParsingFailure}

sealed trait WebsocketRpcError extends EffectError

private[rpc] case class Disconnected(code: Option[Int], reason: String) extends WebsocketRpcError {
  override def getMessage: String = s"closed ${code.getOrElse(" ")} $reason"
}

private[rpc] case class DisconnectedWithError(cause: Throwable) extends WebsocketRpcError with WithCause[Throwable] {
  override def getMessage: String = s"websocket closed due to error: $cause"
}

private[rpc] case class ConnectionFailed(cause: Throwable) extends WebsocketRpcError with WithCause[Throwable] {
  override def getMessage: String = s"connection failed: $cause"
}

private[rpc] case class InvalidJsonResponse(cause: ParsingFailure)
    extends WebsocketRpcError with WithCause[ParsingFailure] {
  override def getMessage: String = s"unable to parse json: $cause"
}

private[rpc] case class InvalidJsonStructure(cause: DecodingFailure)
    extends WebsocketRpcError with WithCause[DecodingFailure] {
  override def getMessage: String = s"can't find required fields in json: $cause"
}

private[rpc] case class BlockParsingFailed(cause: TendermintBlockError, rawBlock: Eval[String], height: Long)
    extends WebsocketRpcError with WithCause[TendermintBlockError] {
  override def getMessage: String =
    s"Websocket TendermintRPC failed to parse block $height: $cause\n" + Console.RED + rawBlock.value + Console.RESET
}

private[rpc] case class BlockRetrievalError(cause: RpcError, height: Long)
    extends WebsocketRpcError with WithCause[RpcError] {
  override def getMessage: String = s"error retrieving block $height from rpc: $cause"
}
