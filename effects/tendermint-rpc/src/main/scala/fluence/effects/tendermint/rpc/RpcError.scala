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

import fluence.effects.EffectError

/** TendermintRpc errors */
sealed trait RpcError extends EffectError

/** Request finished with exception */
case class RpcRequestFailed(cause: Throwable) extends Exception("Tendermint RPC request failed", cause) with RpcError

/** Request was successfully made, but response status is not ok */
case class RpcRequestErrored(statusCode: Int, error: String)
    extends Exception(s"Tendermint RPC request returned error, status=$statusCode, body=$error") with RpcError

/** Response was received, but it's not possible to parse the response to the desired type */
case class RpcBodyMalformed(error: Throwable)
    extends Exception("Tendermint RPC body cannot be parsed", error) with RpcError
