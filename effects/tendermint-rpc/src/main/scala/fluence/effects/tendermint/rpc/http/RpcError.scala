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

package fluence.effects.tendermint.rpc.http

import fluence.effects.tendermint.block.errors.TendermintBlockError
import fluence.effects.{EffectError, WithCause}
import io.circe.Decoder
import io.circe.generic.semiauto.deriveDecoder

/** TendermintHttpRpc errors */
sealed trait RpcError extends EffectError

/** Request finished with exception */
case class RpcRequestFailed(cause: Throwable)
    extends Exception("Tendermint RPC request failed: " + cause.getMessage, cause) with RpcError

/** Request was successfully made, but HTTP response status is not ok */
case class RpcHttpError(statusCode: Int, error: String)
    extends Exception(s"Tendermint RPC request returned error, status=$statusCode, body=$error") with RpcError

/** Request execution returned error */
case class RpcCallError(code: Int, message: String, data: String) extends RpcError {
  override def getMessage: String = s"Error on rpc call: code $code message $message data $data"
}

object RpcCallError {
  import cats.syntax.either._
  implicit val errorDecoder: Decoder[RpcCallError] =
    Decoder.decodeJson.emap(
      _.hcursor
        .downField("error")
        .as[RpcCallError](deriveDecoder[RpcCallError])
        .leftMap(e => s"Error decoding RpcCallError: $e")
    )

  implicit def eitherDecoder[A: Decoder]: Decoder[Either[RpcCallError, A]] =
    errorDecoder.either(implicitly[Decoder[A]])
}

/** Response was received, but it's not possible to parse the response to the desired type */
case class RpcBodyMalformed(request: String, error: Throwable)
    extends Exception(s"Tendermint RPC body cannot be parsed, request: $request", error) with RpcError {
  override def getMessage: String =
    s"Tendermint RPC body cannot be parsed, request: $request $error: ${error.getMessage}"
}

case class RpcBlockParsingFailed(cause: TendermintBlockError, rawBlock: String, height: Long)
    extends RpcError with WithCause[TendermintBlockError] {
  override def getMessage: String =
    s"TendermintRPC failed to parse block $height: $cause\n" + Console.RED + rawBlock + Console.RESET
}
