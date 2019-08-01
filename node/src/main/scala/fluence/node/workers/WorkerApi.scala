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

package fluence.node.workers

import cats.{Apply, Monad}
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.log.Log
import fluence.node.workers.subscription.{TendermintQueryResponse, TxAwaitError}

import scala.language.higherKinds

trait WorkerApi {

  def query[F[_]: Monad](
    worker: Worker[F],
    data: Option[String],
    path: String,
    id: Option[String]
  )(implicit log: Log[F]): F[Either[RpcError, String]]

  def status[F[_]: Monad](worker: Worker[F])(implicit log: Log[F]): F[Either[RpcError, String]]

  def p2pPort[F[_]: Apply](worker: Worker[F])(implicit log: Log[F]): F[Short]

  def sendTx[F[_]: Monad](worker: Worker[F], tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[RpcError, String]]

  def lastManifest[F[_]: Monad](worker: Worker[F]): F[Option[BlockManifest]]

  def sendTxAwaitResponse[F[_]: Monad, G[_]](worker: Worker[F], tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[TxAwaitError, TendermintQueryResponse]]
}

object WorkerApi {
  def apply(): WorkerApi = new WorkerApiImpl()
}
