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
import cats.syntax.applicative._
import cats.syntax.flatMap._
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.effects.tendermint.rpc.http.{RpcBodyMalformed, RpcError}
import fluence.log.Log
import fluence.node.workers.subscription.{TendermintQueryResponse, TxResponseCode}
import fluence.statemachine.data.Tx
import io.circe.parser.decode

import scala.language.higherKinds

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
  def broadcastTx[F[_]: Monad](pool: WorkersPool[F], appId: Long, tx: String, id: Option[String])(
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
   * Returns the last manifest of a worker.
   *
   */
  def lastManifest[F[_]: Monad](pool: WorkersPool[F], appId: Long): F[Option[Option[BlockManifest]]] =
    pool.withWorker(appId, _.withServices(_.blockManifests)(_.lastManifestOpt))

  /**
   * Errors for `txAwait` API
   */
  trait TxAwaitError
  case class RpcTxAwaitError(rpcError: RpcError) extends TxAwaitError
  case class TxParsingError(msg: String, tx: String) extends TxAwaitError
  case class AppNotFoundError(msg: String) extends TxAwaitError
  case class TxInvalidError(msg: String) extends TxAwaitError

  /**
   * Sends the transaction to tendermint and then query for a response after each block.
   *
   * @param pool list of started workers
   * @param appId app id for which the request is intended
   * @param tx transaction to process
   */
  def txAwaitResponse[F[_]: Monad, G[_]](pool: WorkersPool[F], appId: Long, tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[TxAwaitError, TendermintQueryResponse]] =
    log.scope("txWaitResponse.appId" -> appId.toString) { implicit log ⇒
      pool.get(appId).flatMap {
        case Some(worker) =>
          (for {
            _ <- EitherT.right(log.debug(s"TendermintRpc broadcastTxSync in txWaitResponse request"))
            txParsed <- EitherT
              .fromOptionF(Tx.readTx(tx.getBytes()).value,
                           TxParsingError("Incorrect transaction format.", tx): TxAwaitError)
            txBroadcastResponse <- worker.services.tendermint
              .broadcastTxSync(tx, id.getOrElse("dontcare"))
              .leftMap(RpcTxAwaitError(_): TxAwaitError)
            _ <- checkTxResponse(txBroadcastResponse)
            response <- waitResponse(worker, txParsed)
          } yield response).value
        case None =>
          (Left(AppNotFoundError(s"There is no worker with such appId: $appId"): TxAwaitError): Either[
            TxAwaitError,
            TendermintQueryResponse
          ]).pure[F]
      }
    }

  /**
   * Creates a subscription for response and waits when it will be completed.
   *
   */
  private def waitResponse[F[_]: Monad](worker: Worker[F], tx: Tx)(
    implicit log: Log[F]
  ): EitherT[F, TxAwaitError, TendermintQueryResponse] =
    log.scope("txWaitResponse.requestId" -> tx.head.toString) { implicit log =>
      for {
        _ <- EitherT.right(
          log.debug(s"TendermintRpc broadcastTxSync is ok. Waiting for response")
        )
        response <- EitherT.liftF[F, TxAwaitError, TendermintQueryResponse](
          worker.services.responseSubscriber.subscribe(tx.head).flatMap(_.get)
        )
        _ <- Log.eitherT[F, TxAwaitError].trace(s"Response received: $response")
      } yield response
    }

  /**
   * Checks if a response is correct and code value is `ok`. Returns an error otherwise.
   */
  private def checkTxResponse[F[_]: Log](
    response: String
  )(implicit F: Monad[F]): EitherT[F, TxAwaitError, Unit] = {
    for {
      txResponse <- EitherT
        .fromEither[F](decode[TxResponseCode](response))
        .leftMap(err => RpcTxAwaitError(RpcBodyMalformed(err)): TxAwaitError)
      _ <- if (txResponse.code != 0)
        EitherT.left(
          F.pure(
            TxInvalidError(
              s"Response code for transaction is not ok. Code: ${txResponse.code}, info: ${txResponse.info}"
            ): TxAwaitError
          )
        )
      else EitherT.right[TxAwaitError](F.pure(()))
    } yield ()
  }
}
