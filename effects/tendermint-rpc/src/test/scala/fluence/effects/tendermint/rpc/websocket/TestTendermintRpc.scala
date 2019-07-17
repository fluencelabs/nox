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

import cats.Functor
import cats.data.EitherT
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.effects.tendermint.rpc.response.TendermintStatus

trait TestTendermintRpc[F[_]] extends TendermintRpc[F] {
  def status: EitherT[F, RpcError, String] = throw new NotImplementedError("val status")

  def statusParsed(implicit F: Functor[F]): EitherT[F, RpcError, TendermintStatus] =
    throw new NotImplementedError("def statusParsed")

  def block(height: Long, id: String): EitherT[F, RpcError, Block] =
    throw new NotImplementedError("def block")

  def commit(height: Long, id: String): EitherT[F, RpcError, String] =
    throw new NotImplementedError("def commit")

  def consensusHeight(id: String): EitherT[F, RpcError, Long] =
    throw new NotImplementedError("def consensusHeight")

  def broadcastTxSync(tx: String, id: String): EitherT[F, RpcError, String] =
    throw new NotImplementedError("def broadcastTxSync")

  def unsafeDialPeers(peers: Seq[String], persistent: Boolean, id: String): EitherT[F, RpcError, String] =
    throw new NotImplementedError("def unsafeDialPeers")

  def query(
    path: String,
    data: String,
    height: Long,
    prove: Boolean,
    id: String
  ): EitherT[F, RpcError, String] = throw new NotImplementedError("def query")
}
