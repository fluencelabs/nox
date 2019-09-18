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

package fluence.node.workers.subscription

import cats.Monad
import cats.data.EitherT
import cats.effect.Resource
import fluence.log.Log
import cats.syntax.flatMap._
import cats.syntax.functor._
import io.circe.parser.decode
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import cats.syntax.applicative._
import fluence.bp.tx.{Tx, TxCode}

import scala.language.higherKinds

trait WaitResponseService[F[_]] {

  def sendTxAwaitResponse(tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[TxAwaitError, TendermintQueryResponse]]

}

class WaitResponseServiceImpl[F[_]: Monad](
  tendermintRpc: TendermintHttpRpc[F],
  responseSubscriber: ResponseSubscriber[F]
) extends WaitResponseService[F] {

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
        responseSubscriber.subscribe(tx.head).flatMap(_.get)
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

  /**
   * Sends transaction to a state machine and waiting for a response.
   *
   */
  def sendTxAwaitResponse(tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[TxAwaitError, TendermintQueryResponse]] =
    (for {
      _ <- EitherT.right(log.debug(s"TendermintRpc broadcastTxSync in txWaitResponse request"))
      txParsed <- EitherT
        .fromOptionF(Tx.readTx(tx.getBytes()).value, TxParsingError("Incorrect transaction format", tx): TxAwaitError)
      txBroadcastResponse <- tendermintRpc
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

  def start()(implicit log: Log[F]): Resource[F, Unit] = responseSubscriber.start()
}

object WaitResponseService {

  def apply[F[_]: Monad: Log](
    tendermintRpc: TendermintHttpRpc[F],
    responseSubscriber: ResponseSubscriber[F]
  ): Resource[F, WaitResponseService[F]] = {
    val instance = new WaitResponseServiceImpl(tendermintRpc, responseSubscriber)
    instance.start().as(instance)
  }
}
