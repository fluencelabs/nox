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

import cats.Monad
import cats.data.EitherT
import cats.effect.Sync
import cats.syntax.applicative._
import cats.syntax.apply._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.log.Log
import fluence.node.workers.Worker
import fluence.node.workers.api.websocket.WorkerWebsocket
import fluence.node.workers.subscription._
import fluence.statemachine.data.{Tx, TxCode}
import io.circe.parser.decode

import scala.language.higherKinds

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
   * Gets a status of a tendermint node.
   *
   */
  def tendermintStatus()(implicit log: Log[F]): F[Either[RpcError, String]]

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
  def sendTx(tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[RpcError, String]]

  /**
   * Sends the transaction to tendermint and then query for a response after each block.
   *
   * @param tx transaction to process
   */
  def sendTxAwaitResponse(tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[TxAwaitError, TendermintQueryResponse]]

  /**
   * Returns the last manifest of a worker.
   *
   */
  def lastManifest(): F[Option[BlockManifest]]

  def websocket()(implicit log: Log[F]): F[WorkerWebsocket[F]]

  def subscribe(subscriptionId: String, tx: String)(implicit log: Log[F]): F[Either[RpcError, Unit]]
}

object WorkerApi {

  class Impl[F[_]: Sync](worker: Worker[F]) extends WorkerApi[F] {

    override def query(
      data: Option[String],
      path: String,
      id: Option[String]
    )(implicit log: Log[F]): F[Either[RpcError, String]] =
      log.debug(s"TendermintRpc query request. path: $path, data: $data") *>
        worker.withServices(_.tendermintRpc)(_.query(path, data.getOrElse(""), id = id.getOrElse("dontcare")).value)

    override def tendermintStatus()(implicit log: Log[F]): F[Either[RpcError, String]] =
      log.trace(s"TendermintRpc status") *>
        worker.withServices(_.tendermintRpc)(_.status.value)

    override def p2pPort()(implicit log: Log[F]): F[Short] =
      log.trace(s"Worker p2pPort") as worker.p2pPort

    override def lastManifest(): F[Option[BlockManifest]] =
      worker.withServices(_.blockManifests)(_.lastManifestOpt)

    override def sendTx(tx: String, id: Option[String])(
      implicit log: Log[F]
    ): F[Either[RpcError, String]] =
      log.scope("tx" -> tx) { implicit log ⇒
        log.debug(s"TendermintRpc broadcastTxSync request, id: $id") *>
          worker.withServices(_.tendermintRpc)(_.broadcastTxSync(tx, id.getOrElse("dontcare")).value)
      }

    override def sendTxAwaitResponse(tx: String, id: Option[String])(
      implicit log: Log[F]
    ): F[Either[TxAwaitError, TendermintQueryResponse]] =
      (for {
        _ <- EitherT.right(log.debug(s"TendermintRpc broadcastTxSync in txWaitResponse request"))
        txParsed <- EitherT
          .fromOptionF(Tx.readTx(tx.getBytes()).value, TxParsingError("Incorrect transaction format", tx): TxAwaitError)
        txBroadcastResponse <- worker.services.tendermintRpc
          .broadcastTxSync(tx, id.getOrElse("dontcare"))
          .leftMap(RpcTxAwaitError(_): TxAwaitError)
        _ <- Log.eitherT.debug("TendermintRpc broadcastTxSync is ok.")
        response <- log.scope("tx.head" -> txParsed.head.toString) { implicit log =>
          for {
            _ <- checkTxResponse(txBroadcastResponse).recoverWith {
              // Transaction was sent twice, but response should be available, so keep waiting
              case e: TendermintRpcError if e.data.toLowerCase.contains("tx already exists in cache") =>
                Log.eitherT[F, TxAwaitError].warn(s"tx already exists in Tendermint's cache, will wait for response")
            }
            response <- waitResponse(txParsed)
          } yield response
        }
      } yield response).value

    override def websocket()(implicit log: Log[F]): F[WorkerWebsocket[F]] =
      WorkerWebsocket(this)

    /**
     * Creates a subscription for response and waits when it will be completed.
     *
     */
    private def waitResponse(tx: Tx)(
      implicit log: Log[F]
    ): EitherT[F, TxAwaitError, TendermintQueryResponse] =
      for {
        _ <- EitherT.right(log.debug(s"Waiting for response"))
        response <- EitherT.liftF[F, TxAwaitError, TendermintQueryResponse](
          worker.services.responseSubscriber.subscribe(tx.head).flatMap(_.get)
        )
        _ <- Log.eitherT[F, TxAwaitError].trace(s"Response received: $response")
      } yield response

    /**
     * Checks if a response is correct and code value is `ok`. Returns an error otherwise.
     *
     */
    private def checkTxResponse(
      response: String
    )(implicit log: Log[F]): EitherT[F, TxAwaitError, Unit] = {
      for {
        txResponseOrError <- EitherT
          .fromEither[F](decode[Either[TendermintRpcError, TxResponseCode]](response)(TendermintRpcError.eitherDecoder))
          .leftSemiflatMap(
            err =>
              // this is because tendermint could return other responses without code,
              // the node should return this as is to the client
              log
                .error(s"Error on txBroadcastSync response deserialization", err)
                .as(TendermintResponseDeserializationError(response): TxAwaitError)
          )
        txResponse <- EitherT.fromEither[F](txResponseOrError).leftMap(identity[TxAwaitError])
        _ <- if (txResponse.code.exists(_ != TxCode.OK))
          EitherT.left(
            (TxInvalidError(
              s"Response code for transaction is not ok. Code: ${txResponse.code}, info: ${txResponse.info}"
            ): TxAwaitError).pure[F]
          )
        else EitherT.right[TxAwaitError](().pure[F])
      } yield ()
    }

    override def subscribe(subscriptionId: String, tx: String)(implicit log: Log[F]): F[Either[RpcError, Unit]] = ???
  }

  def apply[F[_]: Sync](worker: Worker[F]): WorkerApi[F] = new Impl[F](worker)
}
