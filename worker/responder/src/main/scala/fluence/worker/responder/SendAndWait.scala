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

package fluence.worker.responder

import cats.Monad
import cats.data.EitherT
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.bp.api.BlockProducer
import fluence.bp.tx.{Tx, TxCode, TxResponse}
import fluence.log.Log
import fluence.worker.responder.resp.{AwaitedResponse, RpcTxAwaitError, TxAwaitError, TxInvalidError, TxParsingError}

import scala.language.higherKinds

class SendAndWait[F[_]: Monad](
  producer: BlockProducer[F],
  responseSubscriber: AwaitResponses[F]
) {

  /**
   * Sends transaction to a state machine and waiting for a response.
   *
   */
  def sendTxAwaitResponse(tx: Array[Byte])(
    implicit log: Log[F]
  ): EitherT[F, TxAwaitError, AwaitedResponse] =
    for {
      _ <- EitherT.right(log.debug(s"TendermintRpc broadcastTxSync in txWaitResponse request"))

      txParsed <- EitherT
        .fromOptionF(Tx.readTx(tx).value, TxParsingError("Incorrect transaction format", tx): TxAwaitError)

      txBroadcastResponse <- producer
        .sendTx(tx)
        .leftMap(RpcTxAwaitError(_): TxAwaitError)

      _ <- Log.eitherT.debug("TendermintRpc broadcastTxSync is ok.")
      response <- log.scope("tx.head" -> txParsed.head.toString) { implicit log =>
        for {
          _ <- checkTxResponse(txBroadcastResponse).recoverWith(catchExistingTxError)
          response <- EitherT.right(waitResponse(txParsed))
        } yield response
      }
    } yield response

  /**
   * Creates a subscription for response and waits when it will be completed.
   *
   */
  private def waitResponse(tx: Tx)(
    implicit log: Log[F]
  ): F[AwaitedResponse] =
    for {
      _ <- log.debug(s"Waiting for response")
      response <- responseSubscriber.await(tx.head).flatMap(_.get)
      _ <- Log[F].trace(s"Response received: $response")
    } yield response

  /**
   * Checks if a response is correct and code value is `ok`. Returns an error otherwise.
   *
   */
  private def checkTxResponse(
    txResponse: TxResponse
  )(implicit log: Log[F]): EitherT[F, TxAwaitError, Unit] =
    if (txResponse.code != TxCode.OK)
      EitherT.left(
        (TxInvalidError(
          s"Response code for transaction is not ok. Code: ${txResponse.code}, info: ${txResponse.info}"
        ): TxAwaitError).pure[F]
      )
    else EitherT.right[TxAwaitError](().pure[F])

  private def catchExistingTxError(
    implicit log: Log[F]
  ): PartialFunction[TxAwaitError, EitherT[F, TxAwaitError, Unit]] = {
    // Transaction was sent twice, but response should be available, so keep waiting
    // TODO is it correct? how to handle it better?
    case e if e.msg.toLowerCase.contains("tx already exists in cache") =>
      Log.eitherT[F, TxAwaitError].warn(s"tx already exists in Tendermint's cache, will wait for response")
  }
}

object SendAndWait {

  def apply[F[_]: Monad: Log](
    producer: BlockProducer[F],
    responseSubscriber: AwaitResponses[F]
  ): SendAndWait[F] =
    new SendAndWait(producer, responseSubscriber)
}
