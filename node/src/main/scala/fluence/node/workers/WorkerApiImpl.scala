package fluence.node.workers

import cats.{Apply, Monad}
import cats.syntax.apply._
import cats.syntax.applicative._
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.data.EitherT
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.log.Log
import fluence.node.workers.subscription.{
  RpcTxAwaitError,
  TendermintQueryResponse,
  TendermintResponseError,
  TxAwaitError,
  TxInvalidError,
  TxParsingError,
  TxResponseCode
}
import fluence.statemachine.data.{Tx, TxCode}
import io.circe.parser.decode

import scala.language.higherKinds

/**
 * API that independent from transports.
 */
class WorkerApiImpl extends WorkerApi {

  /**
   * Sends `query` request to tendermint.
   *
   * @param data body of the request
   * @param path id of a response
   */
  def query[F[_]: Monad](
    worker: Worker[F],
    data: Option[String],
    path: String,
    id: Option[String]
  )(implicit log: Log[F]): F[Either[RpcError, String]] =
    log.debug(s"TendermintRpc query request. path: $path, data: $data") *>
      worker.withServices(_.tendermint)(_.query(path, data.getOrElse(""), id = id.getOrElse("dontcare")).value)

  /**
   * Gets a status of a tendermint node.
   *
   */
  def status[F[_]: Monad](worker: Worker[F])(implicit log: Log[F]): F[Either[RpcError, String]] =
    log.trace(s"TendermintRpc status") *>
      worker.withServices(_.tendermint)(_.status.value)

  /**
   * Gets a p2p port of tendermint.
   *
   */
  def p2pPort[F[_]: Apply](worker: Worker[F])(implicit log: Log[F]): F[Short] =
    log.debug(s"Worker p2pPort") as
      worker.p2pPort

  /**
   * Sends transaction to tendermint.
   *
   * @param tx transaction to process
   */
  def sendTx[F[_]: Monad](worker: Worker[F], tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[RpcError, String]] =
    log.scope("tx.id" -> tx) { implicit log ⇒
      log.debug(s"TendermintRpc broadcastTxSync request, id: $id") *>
        worker.withServices(_.tendermint)(_.broadcastTxSync(tx, id.getOrElse("dontcare")).value)
    }

  /**
   * Returns the last manifest of a worker.
   *
   */
  def lastManifest[F[_]: Monad](worker: Worker[F]): F[Option[BlockManifest]] =
    worker.withServices(_.blockManifests)(_.lastManifestOpt)

  /**
   * Sends the transaction to tendermint and then query for a response after each block.
   *
   * @param tx transaction to process
   */
  def sendTxAwaitResponse[F[_]: Monad, G[_]](worker: Worker[F], tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[TxAwaitError, TendermintQueryResponse]] =
    (for {
      _ <- EitherT.right(log.debug(s"TendermintRpc broadcastTxSync in txWaitResponse request"))
      txParsed <- EitherT
        .fromOptionF(Tx.readTx(tx.getBytes()).value, TxParsingError("Incorrect transaction format", tx): TxAwaitError)
      txBroadcastResponse <- worker.services.tendermint
        .broadcastTxSync(tx, id.getOrElse("dontcare"))
        .leftMap(RpcTxAwaitError(_): TxAwaitError)
      response <- log.scope("sessionId" -> txParsed.head.toString) { implicit log =>
        for {
          _ <- checkTxResponse(txBroadcastResponse)
          response <- waitResponse(worker, txParsed)
        } yield response
      }
    } yield response).value

  /**
   * Creates a subscription for response and waits when it will be completed.
   *
   */
  private def waitResponse[F[_]: Monad](worker: Worker[F], tx: Tx)(
    implicit log: Log[F]
  ): EitherT[F, TxAwaitError, TendermintQueryResponse] =
    for {
      _ <- EitherT.right(
        log.debug(s"TendermintRpc broadcastTxSync is ok. Waiting for response")
      )
      response <- EitherT.liftF[F, TxAwaitError, TendermintQueryResponse](
        worker.services.responseSubscriber.subscribe(tx.head).flatMap(_.get)
      )
      _ <- Log.eitherT[F, TxAwaitError].trace(s"Response received: $response")
    } yield response

  /**
   * Checks if a response is correct and code value is `ok`. Returns an error otherwise.
   */
  private def checkTxResponse[F[_]: Monad](
    response: String
  )(implicit log: Log[F]): EitherT[F, TxAwaitError, Unit] = {
    for {
      txResponse <- EitherT
        .fromEither[F](decode[TxResponseCode](response))
        .leftSemiflatMap(
          err =>
            // this is because tendermint could return other responses without code,
            // the node should return this as is to the client
            log
              .error(s"Error on txBroadcastSync response deserialization", err)
              .as(TendermintResponseError(response): TxAwaitError)
        )
      _ <- if (txResponse.code.exists(_ != TxCode.OK))
        EitherT.left(
          (TxInvalidError(
            s"Response code for transaction is not ok. Code: ${txResponse.code}, info: ${txResponse.info}"
          ): TxAwaitError).pure[F]
        )
      else EitherT.right[TxAwaitError](().pure[F])
    } yield ()
  }
}
