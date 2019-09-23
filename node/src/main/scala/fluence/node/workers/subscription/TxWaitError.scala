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

package fluence.node.workers.subscription

import fluence.effects.tendermint.rpc.http.RpcError
import io.circe.Decoder
import io.circe.generic.semiauto.deriveDecoder

import scala.util.control.NoStackTrace

/**
 * Errors for `txAwait` API
 */
trait TxAwaitError {
  def msg: String
}
case class TendermintResponseDeserializationError(responseError: String) extends TxAwaitError {
  override def msg: String = responseError
}
case class RpcTxAwaitError(rpcError: RpcError) extends TxAwaitError {
  override def msg: String = rpcError.getMessage
}
case class TxParsingError(msg: String, tx: Array[Byte]) extends TxAwaitError
case class TxInvalidError(msg: String) extends TxAwaitError
case class TendermintRpcError(code: Int, message: String, data: String) extends TxAwaitError {
  override def msg: String = s"Tendermint error. Code: $code, message: $message, data: $data"
}

object TendermintRpcError {
  import cats.syntax.either._
  implicit val errorDecoder: Decoder[TendermintRpcError] =
    Decoder.decodeJson.emap(
      _.hcursor
        .downField("error")
        .as[TendermintRpcError](deriveDecoder[TendermintRpcError])
        .leftMap(e => s"Error decoding TendermintError: $e")
    )

  implicit def eitherDecoder[A: Decoder]: Decoder[Either[TendermintRpcError, A]] =
    errorDecoder.either(implicitly[Decoder[A]])
}
