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
import cats.effect.Resource
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.effects.tendermint.rpc.response.TendermintStatus
import fluence.effects.{Backoff, EffectError}
import fluence.log.Log
import fs2.concurrent.Queue

import scala.language.higherKinds

trait TestTendermintRpc[F[_]] extends TendermintWebsocketRpc[F] with TendermintRpc[F] {
  override def status: EitherT[F, RpcError, String] = throw new NotImplementedError("val status")

  override def statusParsed(implicit F: Functor[F]): EitherT[F, RpcError, TendermintStatus] =
    throw new NotImplementedError("def statusParsed")

  override def block(height: Long, id: String): EitherT[F, RpcError, Block] =
    throw new NotImplementedError("def block")

  override def commit(height: Long, id: String): EitherT[F, RpcError, String] =
    throw new NotImplementedError("def commit")

  override def consensusHeight(id: String): EitherT[F, RpcError, Long] =
    throw new NotImplementedError("def consensusHeight")

  override def broadcastTxSync(tx: String, id: String): EitherT[F, RpcError, String] =
    throw new NotImplementedError("def broadcastTxSync")

  override def unsafeDialPeers(peers: Seq[String], persistent: Boolean, id: String): EitherT[F, RpcError, String] =
    throw new NotImplementedError("def unsafeDialPeers")

  override def query(
    path: String,
    data: String,
    height: Long,
    prove: Boolean,
    id: String
  ): EitherT[F, RpcError, String] = throw new NotImplementedError("def query")

  override def subscribeNewBlock(lastKnownHeight: Long)(implicit log: Log[F],
                                                        backoff: Backoff[EffectError]): fs2.Stream[F, Block] =
    throw new NotImplementedError("def subscribeNewBlock")

  override protected def subscribe(event: String)(implicit log: Log[F],
                                                  backoff: Backoff[EffectError]): Resource[F, Queue[F, Event]] =
    throw new NotImplementedError("def subscribe")
}
