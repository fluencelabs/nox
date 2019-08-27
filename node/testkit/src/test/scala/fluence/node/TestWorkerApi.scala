package fluence.node

import cats.{Apply, Monad}
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.log.Log
import fluence.node.workers.subscription.{TendermintQueryResponse, TxAwaitError}
import fluence.node.workers.{Worker, WorkerApi}

import scala.language.higherKinds

class TestWorkerApi extends WorkerApi {

  /**
   * Sends `query` request to tendermint.
   *
   * @param data body of the request
   * @param path id of a response
   */
  override def query[F[_]: Monad](worker: Worker[F], data: Option[String], path: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[RpcError, String]] = ???

  /**
   * Gets a status of a tendermint node.
   *
   */
  override def status[F[_]: Monad](worker: Worker[F])(implicit log: Log[F]): F[Either[RpcError, String]] = ???

  /**
   * Gets a p2p port of tendermint node.
   *
   */
  override def p2pPort[F[_]: Monad](worker: Worker[F])(implicit log: Log[F]): F[Short] = ???

  /**
   * Sends transaction to tendermint broadcastTxSync.
   *
   * @param tx transaction to process
   */
  override def sendTx[F[_]: Monad](worker: Worker[F], tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[RpcError, String]] = ???

  /**
   * Sends the transaction to tendermint and then query for a response after each block.
   *
   * @param tx transaction to process
   */
  override def sendTxAwaitResponse[F[_]: Monad, G[_]](worker: Worker[F], tx: String, id: Option[String])(
    implicit log: Log[F]
  ): F[Either[TxAwaitError, TendermintQueryResponse]] = ???

  /**
   * Returns the last manifest of a worker.
   *
   */
  override def lastManifest[F[_]: Monad](worker: Worker[F]): F[Option[BlockManifest]] = ???
}
