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
import fluence.node.workers.subscription.{RequestSubscriber, TendermintQueryResponse}
import fluence.statemachine.data.Tx
import io.circe.parser.decode

import scala.language.higherKinds

trait TxSyncResponse
trait TxSyncErrorT
case class RpcTxSyncError(rpcError: RpcError) extends TxSyncErrorT
case class TxSyncError(msg: String, responseBody: String) extends TxSyncErrorT

object WorkersApi {

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

  def status[F[_]: Monad](pool: WorkersPool[F],
                          appId: Long)(implicit log: Log[F]): F[Option[Either[RpcError, String]]] =
    log.trace(s"TendermintRpc status") *>
      pool.withWorker(
        appId,
        _.withServices(_.tendermint)(_.status.value)
      )

  def p2pPort[F[_]: Apply](pool: WorkersPool[F], appId: Long)(implicit log: Log[F]): F[Option[Worker[F]]] =
    log.debug(s"Worker p2pPort") *>
      pool.get(appId)

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

  def txWaitResponse[F[_]: Monad, G[_]](pool: WorkersPool[F],
                                        requestSubscriber: RequestSubscriber[F],
                                        appId: Long,
                                        tx: String,
                                        id: Option[String])(
    implicit log: Log[F]
  ): F[Either[TxSyncErrorT, TendermintQueryResponse]] =
    log.scope("txWaitResponse.id" -> tx) { implicit log ⇒
      log.debug(s"TendermintRpc broadcastTxSync in txWaitResponse request, id: $id")
      (for {
        response <- withTendermintRaw(pool, appId)(
          _.broadcastTxSync(tx, id.getOrElse("dontcare"))
        ).leftMap(RpcTxSyncError(_): TxSyncErrorT)
        tx <- checkResponseParseRequest(response, tx)
        response <- EitherT.liftF[F, TxSyncErrorT, TendermintQueryResponse](
          requestSubscriber.subscribe(appId, tx.head).flatMap(_.get)
        )
      } yield response).value
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

  private def checkResponseParseRequest[F[_]: Log](
    responseOp: Option[String],
    request: String
  )(implicit F: Monad[F]): EitherT[F, TxSyncErrorT, Tx] = {
    for {
      _ <- if (responseOp.isEmpty)
        EitherT.left(F.pure(TxSyncError("There is no worker with such appId", ""): TxSyncErrorT))
      else EitherT.pure[F, TxSyncErrorT](())
      response = responseOp.get
      code <- EitherT
        .fromEither[F](decode[TxResponseCode](response))
        .leftMap(err => RpcTxSyncError(RpcBodyMalformed(err)): TxSyncErrorT)
        .map(_.code)
      tx <- if (code != 0) EitherT.left(F.pure(TxSyncError("Transaction is not ok", response): TxSyncErrorT))
      else
        EitherT
          .fromOptionF(Tx.readTx(request.getBytes()).value, TxSyncError("Cannot parse tx", response): TxSyncErrorT)
    } yield tx
  }
}
