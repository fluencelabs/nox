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

import cats.Functor
import cats.data.EitherT
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.response.TendermintStatus

import scala.language.higherKinds

/**
 * Algebra for Tendermint's HTTP rpc
 */
trait TendermintHttpRpc[F[_]] {

  /** Gets status as a string */
  def status: EitherT[F, RpcError, String]

  /** Gets status, parse it to [[TendermintStatus]] */
  def statusParsed(implicit F: Functor[F]): EitherT[F, RpcError, TendermintStatus]

  /** Retrieves a block at the given height */
  def block(height: Long, id: String = "dontcare"): EitherT[F, RpcError, Block]

  /** Retireves a commit at the given height */
  def commit(height: Long, id: String = "dontcare"): EitherT[F, RpcError, String]

  /**
   * Returns last block height known by this Tendermint node
   */
  def consensusHeight(id: String = "dontcare"): EitherT[F, RpcError, Long]

  /** Sends a transaction to the Tendermint node */
  def broadcastTxSync(tx: String, id: String): EitherT[F, RpcError, String]

  /**
   * Signals Tendermint node to connecting to the specified peers
   *
   * @param peers Peers to connect to
   * @param persistent If true, store them persitently, and always try to reconnect with them
   */
  def unsafeDialPeers(
    peers: Seq[String],
    persistent: Boolean,
    id: String = "dontcare"
  ): EitherT[F, RpcError, String]

  def query(
    path: String,
    data: String = "",
    height: Long = 0,
    prove: Boolean = false,
    id: String
  ): EitherT[F, RpcError, String]
}
