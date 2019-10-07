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
import fluence.bp.tx.{Tx, TxResponse}
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.log.Log
import fluence.node.workers.api.WorkerApi
import fluence.node.workers.api.websocket.WorkerWebsocket
import fluence.worker.responder.repeat.SubscriptionKey
import fluence.worker.responder.resp.AwaitedResponse.OrError

import scala.language.higherKinds

class TestWorkerApi[F[_]: Concurrent]() extends WorkerApi[F] {

  override def query(data: Option[String], path: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[RpcError, String]] =
    throw new NotImplementedError("TestWorkerApi, method query")

  override def sendTx(tx: Array[Byte])(implicit log: Log[F]): F[Either[RpcError, TxResponse]] =
    throw new NotImplementedError("TestWorkerApi, method sendTx")

  override def sendTxAwaitResponse(tx: Array[Byte])(implicit log: Log[F]): F[OrError] =
    throw new NotImplementedError("TestWorkerApi, method sendTxAwaitResponse")

  override def subscribe(key: SubscriptionKey, tx: Tx.Data)(implicit log: Log[F]): F[fs2.Stream[F, OrError]] =
    throw new NotImplementedError("TestWorkerApi, method subscribe")

  override def unsubscribe(key: SubscriptionKey)(implicit log: Log[F]): F[Boolean] =
    throw new NotImplementedError("TestWorkerApi, method unsubscribe")

  override def websocket()(implicit log: Log[F]): F[WorkerWebsocket[F]] = WorkerWebsocket(this)
}
