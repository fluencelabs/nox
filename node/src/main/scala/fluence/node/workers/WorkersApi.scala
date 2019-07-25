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

import cats.data.EitherT
import cats.{Apply, Monad}
import cats.syntax.apply._
import cats.syntax.functor._
import cats.syntax.flatMap._
import fluence.effects.tendermint.rpc.{TendermintRpc, TxResponseCode}
import fluence.effects.tendermint.rpc.http.{RpcBodyMalformed, RpcError}
import fluence.log.Log
import fluence.node.workers.subscription.{RequestResponder, TendermintQueryResponse}
import fluence.statemachine.data.Tx
import io.circe.parser.decode

import scala.language.higherKinds

trait TxAwaitError
case class RpcTxAwaitError(rpcError: RpcError) extends TxAwaitError
case class TxSyncError(msg: String, responseBody: String) extends TxAwaitError

object WorkersApi {

  /**
   * Sends `query` request to tendermint.
   *
   * @param pool list of started workers
   * @param appId app id for which the request is intended
   * @param data body of the request
   * @param path id of a response
   */
  def query[F[_]: Monad](
    pool: WorkersPool[F],
    appId: Long,
    data: Option[String],
    path: String,
    id: Option[String]
  )(implicit log: Log[F]): F[Option[Either[RpcError, String]]] =
    log.debug(s"TendermintRpc query request. path: $path, data: $data") *>
      pool.withWorker(
        appId,
        _.withServices(_.tendermint)(_.query(path, data.getOrElse(""), id = id.getOrElse("dontcare")).value)
      )

  /**
   * Gets a status of a tendermint node.
   *
   * @param pool list of started workers
   * @param appId app id for which the request is intended
   */
  def status[F[_]: Monad](pool: WorkersPool[F],
                          appId: Long)(implicit log: Log[F]): F[Option[Either[RpcError, String]]] =
    log.trace(s"TendermintRpc status") *>
      pool.withWorker(
        appId,
        _.withServices(_.tendermint)(_.status.value)
      )

  /**
   * Gets a p2p port of tendermint.
   *
   * @param pool list of started workers
   * @param appId app id for which the request is intended
   */
  def p2pPort[F[_]: Apply](pool: WorkersPool[F], appId: Long)(implicit log: Log[F]): F[Option[Worker[F]]] =
    log.debug(s"Worker p2pPort") *>
      pool.get(appId)

  /**
   * Sends transaction to tendermint.
   *
   * @param pool list of started workers
   * @param appId app id for which the request is intended
   * @param tx transaction to process
   */
  def txSync[F[_]: Monad](pool: WorkersPool[F], appId: Long, tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Option[Either[RpcError, String]]] =
    log.scope("tx.id" -> tx) { implicit log ⇒
      log.debug(s"TendermintRpc broadcastTxSync request, id: $id") *>
        pool.withWorker(
          appId,
          _.withServices(_.tendermint)(_.broadcastTxSync(tx, id.getOrElse("dontcare")).value)
        )
    }

  /**
   * Sends the transaction to tendermint and then query for a response after each block.
   *
   * @param pool list of started workers
   * @param requestResponder service that creates request subscriptions and completes them after a response is ready
   * @param appId app id for which the request is intended
   * @param tx transaction to process
   */
  def txWaitResponse[F[_]: Monad, G[_]](pool: WorkersPool[F],
                                        requestResponder: RequestResponder[F],
                                        appId: Long,
                                        tx: String,
                                        id: Option[String])(
    implicit log: Log[F]
  ): F[Either[TxAwaitError, TendermintQueryResponse]] =
    log.scope("txWaitResponse.appId" -> appId.toString) { implicit log ⇒
      (for {
        _ <- EitherT.right(log.debug(s"TendermintRpc broadcastTxSync in txWaitResponse request"))
        txSyncResponse <- withTendermintRaw(pool, appId)(
          _.broadcastTxSync(tx, id.getOrElse("dontcare"))
        ).leftMap(RpcTxAwaitError(_): TxAwaitError)
        _ <- checkTxResponse(txSyncResponse)
        tx <- EitherT
          .fromOptionF(Tx.readTx(tx.getBytes()).value, TxSyncError("Cannot parse tx", txSyncResponse.get): TxAwaitError)
        response <- waitResponse(requestResponder, appId, tx)
      } yield response).value
    }

  private def waitResponse[F[_]: Monad](requestResponder: RequestResponder[F], appId: Long, tx: Tx)(
    implicit log: Log[F]
  ): EitherT[F, TxAwaitError, TendermintQueryResponse] =
    log.scope("txWaitResponse.requestId" -> tx.head.toString) { implicit log =>
      for {
        _ <- EitherT.right(
          log.debug(s"TendermintRpc broadcastTxSync is ok. Id: ${tx.head}. Waiting for response")
        )
        response <- EitherT.liftF[F, TxAwaitError, TendermintQueryResponse](
          requestResponder.subscribe(appId, tx.head).flatMap(_.get)
        )
        _ <- EitherT.right(
          log.debug(s"Response received")
        )
      } yield response
    }

  private def withTendermintRaw[F[_]: Monad](
    pool: WorkersPool[F],
    appId: Long
  )(
    fn: TendermintRpc[F] ⇒ EitherT[F, RpcError, String]
  )(implicit log: Log[F]): EitherT[F, RpcError, Option[String]] =
    EitherT(
      pool
        .withWorker(appId, _.withServices(_.tendermint)(fn(_).value))
        .map(_.fold[Either[RpcError, Option[String]]](Right(None))(_.map(Some(_))))
    )

  private def checkTxResponse[F[_]: Log](
    responseOp: Option[String]
  )(implicit F: Monad[F]): EitherT[F, TxAwaitError, Unit] = {
    for {
      _ <- if (responseOp.isEmpty)
        EitherT.left(F.pure(TxSyncError("There is no worker with such appId", ""): TxAwaitError))
      else EitherT.pure[F, TxAwaitError](())
      response = responseOp.get
      code <- EitherT
        .fromEither[F](decode[TxResponseCode](response))
        .leftMap(err => RpcTxAwaitError(RpcBodyMalformed(err)): TxAwaitError)
        .map(_.code)
      _ <- if (code != 0) EitherT.left(F.pure(TxSyncError("Transaction is not ok", response): TxAwaitError))
      else EitherT.right[TxAwaitError](F.pure(()))
    } yield ()
  }
}
