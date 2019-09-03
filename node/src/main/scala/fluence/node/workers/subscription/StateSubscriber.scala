package fluence.node.workers.subscription

import cats.effect.Resource
import fluence.statemachine.data.Tx
import scodec.bits.ByteVector

import scala.language.higherKinds

trait StateSubscriber[F[_]] {

  /**
   * Makes a subscription by transaction.
   * The master node will send a transaction to state machine after every block
   * and will return response to a connected client.
   *
   * @param data a transaction
   * @return a stream of responses every block
   */
  def subscribe(data: Tx.Data): F[fs2.Stream[F, Either[TxAwaitError, TendermintQueryResponse]]]

  def unsubscribe(data: Tx.Data): F[Boolean]

  /**
   * Gets all transaction subscribes for appId and trying to poll service for new responses.
   *
   */
  def start(): Resource[F, Unit]
}
