package fluence.node.workers.subscription

import cats.Monad
import cats.effect.concurrent.Ref
import cats.effect.{Concurrent, Resource, Sync, Timer}
import fluence.crypto.Crypto.Hasher
import fluence.effects.tendermint.rpc.http.TendermintHttpRpc
import fluence.effects.tendermint.rpc.websocket.TendermintWebsocketRpc
import fluence.statemachine.data.Tx
import cats.syntax.functor._
import fluence.log.Log
import fluence.node.workers.subscription.StoredProcedureExecutor.{Event, TendermintResponse}

import scala.language.higherKinds

/**
 * Service to call subscribed transactions after each tendermint block.
 *
 */
trait StoredProcedureExecutor[F[_]] {

  /**
   * Makes a subscription by transaction.
   * The master node will send a transaction to state machine after every block
   * and will return response to a connected client.
   *
   * @param data a transaction
   * @return a stream of responses every block
   */
  def subscribe(subscriberId: String, data: Tx.Data): F[fs2.Stream[F, TendermintResponse]]

  def unsubscribe(subscriberId: String, data: Tx.Data): F[Boolean]

  /**
   * Gets all transaction subscribes for appId and trying to poll service for new responses.
   *
   */
  def start(): Resource[F, Unit]
}

object StoredProcedureExecutor {

  sealed trait Event
  case class Response(value: TendermintResponse) extends Event
  case object Init extends Event
  case class Quit(id: String) extends Event

  type TendermintResponse = Either[TxAwaitError, TendermintQueryResponse]

  def apply[F[_]: Monad: Timer: Sync: Concurrent: Log](
    tendermintWRpc: TendermintWebsocketRpc[F],
    tendermintRpc: TendermintHttpRpc[F],
    waitResponseService: WaitResponseService[F],
    hasher: Hasher[Array[Byte], String]
  ): F[StoredProcedureExecutor[F]] =
    for {
      subs <- Ref.of[F, Map[String, SubscriptionState[F]]](Map.empty)
    } yield new StoredProcedureExecutorImpl[F](subs, tendermintWRpc, tendermintRpc, waitResponseService, hasher)

  def make[F[_]: Monad: Timer: Sync: Concurrent: Log](
    tendermintWRpc: TendermintWebsocketRpc[F],
    tendermintRpc: TendermintHttpRpc[F],
    waitResponseService: WaitResponseService[F],
    hasher: Hasher[Array[Byte], String]
  ): Resource[F, StoredProcedureExecutor[F]] =
    Resource.liftF(apply(tendermintWRpc, tendermintRpc, waitResponseService, hasher))
}
