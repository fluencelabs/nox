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

import cats.data.EitherT
import cats.effect.{ConcurrentEffect, ContextShift, Timer}
import cats.syntax.applicative._
import cats.syntax.either._
import cats.{Functor, Monad}
import fluence.effects.syntax.eitherT._
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.http.{RpcError, TendermintHttpRpc}
import fluence.effects.tendermint.rpc.response.TendermintStatus

import scala.language.higherKinds

class TestHttpRpc[F[_]: ConcurrentEffect: Timer: Monad: ContextShift] extends TendermintHttpRpc[F] {
  override def status: EitherT[F, RpcError, String] = throw new NotImplementedError("val status")

  override def statusParsed(implicit F: Functor[F]): EitherT[F, RpcError, TendermintStatus] =
    throw new NotImplementedError("def statusParsed")

  override def block(height: Long, id: String): EitherT[F, RpcError, Block] =
    throw new NotImplementedError(s"def block $height")

  override def commit(height: Long, id: String): EitherT[F, RpcError, String] =
    throw new NotImplementedError("def commit")

  override def consensusHeight(id: String): EitherT[F, RpcError, Long] = 0L.asRight[RpcError].pure[F].eitherT

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
}
