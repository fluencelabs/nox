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
import fluence.statemachine.data.Tx

// possible variants of responses from tendermint's `query` method
trait TendermintQueryResponse {
  def id: Tx.Head
}

/**
 * Response that is ok for client. Master node must return it right away.
 *
 */
case class OkResponse(id: Tx.Head, body: String) extends TendermintQueryResponse

/**
 * Transport error in Tendermint RPC.
 *
 */
case class RpcErrorResponse(id: Tx.Head, error: RpcError) extends TendermintQueryResponse

/**
 * Response is not ready yet in state machine.
 *
 */
case class TimedOutResponse(id: Tx.Head, body: String) extends TendermintQueryResponse
