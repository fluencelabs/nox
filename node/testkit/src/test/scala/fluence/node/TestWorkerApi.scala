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

package fluence.node

import cats.effect.Concurrent
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.log.Log
import fluence.node.workers.api.WorkerApi
import fluence.node.workers.api.websocket.WorkerWebsocket
import fluence.node.workers.api.websocket.WorkerWebsocket.SubscriptionKey
import fluence.node.workers.subscription.PerBlockTxExecutor.TendermintResponse
import fluence.statemachine.api.tx.Tx

import scala.language.higherKinds

class TestWorkerApi[F[_]: Concurrent]() extends WorkerApi[F] {

  /**
   * Sends `query` request to tendermint.
   *
   * @param data body of the request
   * @param path id of a response
   */
  override def query(data: Option[String], path: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[RpcError, String]] = throw new NotImplementedError("TestWorkerApi, method query")

  /**
   * Gets a status of a tendermint node.
   *
   */
  override def tendermintStatus()(implicit log: Log[F]): F[Either[RpcError, String]] =
    throw new NotImplementedError("TestWorkerApi, method status")

  /**
   * Gets a p2p port of tendermint node.
   *
   */
  override def p2pPort()(implicit log: Log[F]): F[Short] =
    throw new NotImplementedError("TestWorkerApi, method p2pPort")

  /**
   * Sends transaction to tendermint broadcastTxSync.
   *
   * @param tx transaction to process
   */
  override def sendTx(tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[RpcError, String]] = throw new NotImplementedError("TestWorkerApi, method sendTx")

  /**
   * Sends the transaction to tendermint and then query for a response after each block.
   *
   * @param tx transaction to process
   */
  override def sendTxAwaitResponse(tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[TendermintResponse] =
    throw new NotImplementedError("TestWorkerApi, method sendTxAwaitResponse")

  /**
   * Returns the last manifest of a worker.
   *
   */
  override def lastManifest(): F[Option[BlockManifest]] =
    throw new NotImplementedError("TestWorkerApi, method lastManifest")

  override def websocket()(implicit log: Log[F]): F[WorkerWebsocket[F]] = WorkerWebsocket(this)

  override def subscribe(key: SubscriptionKey, tx: Tx.Data)(
    implicit log: Log[F]
  ): F[fs2.Stream[F, TendermintResponse]] = throw new NotImplementedError("TestWorkerApi, method subscribe")

  override def unsubscribe(key: SubscriptionKey)(
    implicit log: Log[F]
  ): F[Boolean] = throw new NotImplementedError("TestWorkerApi, method unsubscribe")
}
