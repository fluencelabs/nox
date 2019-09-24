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

package fluence.node.workers.api

import cats.effect.Concurrent
import cats.syntax.apply._
import cats.syntax.functor._
import fluence.bp.tx.{Tx, TxResponse}
import fluence.effects.tendermint.rpc.http.{RpcError, RpcRequestFailed}
import fluence.log.Log
import fluence.node.workers.Worker
import fluence.node.workers.api.websocket.WorkerWebsocket
import fluence.worker.responder.repeat.SubscriptionKey
import fluence.worker.responder.resp.AwaitedResponse

import scala.language.higherKinds

// TODO rewrite all errors!
trait WorkerApi[F[_]] {

  /**
   * Sends `query` request to tendermint.
   *
   * @param data body of the request
   * @param path id of a response
   */
  def query(
    data: Option[String],
    path: String,
    id: Option[String]
  )(implicit log: Log[F]): F[Either[RpcError, String]]

  /**
   * Gets a p2p port of tendermint node.
   *
   */
  def p2pPort()(implicit log: Log[F]): F[Short]

  /**
   * Sends transaction to tendermint broadcastTxSync.
   *
   * @param tx transaction to process
   */
  def sendTx(tx: Array[Byte])(implicit log: Log[F]): F[Either[RpcError, TxResponse]]

  /**
   * Sends the transaction to tendermint and then query for a response after each block.
   *
   * @param tx transaction to process
   */
  def sendTxAwaitResponse(tx: Array[Byte])(
    implicit log: Log[F]
  ): F[AwaitedResponse.OrError]

  /**
   * Creates service to work with websocket
   *
   */
  def websocket()(implicit log: Log[F]): F[WorkerWebsocket[F]]

  /**
   * Subscribes on the transaction processing after each block.
   *
   * @param key an id of subscription
   * @param tx a transaction that will be executed on state machine after each block
   * @return a stream with responses on transactions for each block
   */
  def subscribe(key: SubscriptionKey, tx: Tx.Data)(
    implicit log: Log[F]
  ): F[fs2.Stream[F, AwaitedResponse.OrError]]

  /**
   * Remove given subscription.
   *
   */
  def unsubscribe(key: SubscriptionKey)(
    implicit log: Log[F]
  ): F[Boolean]
}

object WorkerApi {

  class Impl[F[_]: Concurrent](worker: Worker[F]) extends WorkerApi[F] {

    override def query(
      data: Option[String],
      path: String,
      id: Option[String]
    )(implicit log: Log[F]): F[Either[RpcError, String]] =
      log.debug(s"TendermintRpc query request. path: $path, data: $data") *>
        worker.withServices(_.machine)(
          _.query(path)
            .leftMap(e ⇒ RpcRequestFailed(e): RpcError)
            .map(_.toResponseString(id.getOrElse("dontcare")))
            .value
        )

    override def p2pPort()(implicit log: Log[F]): F[Short] =
      log.trace(s"Worker p2pPort") as worker.p2pPort

    override def sendTx(tx: Array[Byte])(
      implicit log: Log[F]
    ): F[Either[RpcError, TxResponse]] =
      log.scope("tx") { implicit log ⇒
        log.debug(s"TendermintRpc broadcastTxSync request") *>
          worker.withServices(_.producer)(_.sendTx(tx).leftMap[RpcError](RpcRequestFailed(_)).value)
      }

    override def sendTxAwaitResponse(tx: Array[Byte])(
      implicit log: Log[F]
    ): F[AwaitedResponse.OrError] =
      log.scope("txWait") { implicit log ⇒
        worker.withServices(_.responder.sendAndWait)(_.sendTxAwaitResponse(tx).value)
      }

    override def websocket()(implicit log: Log[F]): F[WorkerWebsocket[F]] =
      WorkerWebsocket(this)

    override def subscribe(key: SubscriptionKey, tx: Tx.Data)(
      implicit log: Log[F]
    ): F[fs2.Stream[F, AwaitedResponse.OrError]] =
      log.scope("subscriptionKey" -> key.toString) { implicit log ⇒
        worker.withServices(_.responder.onEveryBlock)(
          _.subscribe(key, tx)
        )
      }

    override def unsubscribe(key: SubscriptionKey)(implicit log: Log[F]): F[Boolean] =
      log.scope("subscriptionKey" -> key.toString) { implicit log ⇒
        worker.withServices(_.responder.onEveryBlock)(
          _.unsubscribe(key)
        )
      }
  }

  def apply[F[_]: Concurrent](worker: Worker[F]): WorkerApi[F] = new Impl[F](worker)
}
