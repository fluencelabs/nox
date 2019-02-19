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

package fluence.node.workers.tendermint.rpc
import io.circe.generic.semiauto.deriveEncoder
import io.circe.{Encoder, Json}

/**
 * Wrapper for Tendermint's RPC request
 *
 * @param method Method name
 * @param jsonrpc Version of the JSON RPC protocol
 * @param params Sequence of arguments for the method
 * @param id Nonce to track the results of the request with some other method
 */
case class RpcRequest(method: String, jsonrpc: String = "2.0", params: Seq[Json], id: String = "") {
  def toJsonString: String = RpcRequest.requestEncoder(this).noSpaces
}

object RpcRequest {
  val requestEncoder: Encoder[RpcRequest] = deriveEncoder[RpcRequest]
}
