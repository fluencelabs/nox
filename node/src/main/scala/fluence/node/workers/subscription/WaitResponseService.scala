package fluence.node.workers.subscription

import cats.Monad
import cats.data.EitherT
import cats.effect.Resource
import fluence.log.Log
import fluence.statemachine.data.{Tx, TxCode}
import cats.syntax.flatMap._
import cats.syntax.functor._
import io.circe.parser.decode
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import cats.syntax.applicative._

import scala.language.higherKinds

class WaitResponseService[F[_]: Monad](tendermintRpc: TendermintHttpRpc[F], responseSubscriber: ResponseSubscriber[F]) {

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

  def start(): Resource[F, Unit] = responseSubscriber.start()
}

object WaitResponseService {

  def apply[F[_]: Monad](
    tendermintRpc: TendermintHttpRpc[F],
    responseSubscriber: ResponseSubscriber[F]
  ): WaitResponseService[F] = new WaitResponseService(tendermintRpc, responseSubscriber)
}
