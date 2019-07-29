package fluence.node

import cats.Monad
import cats.data.EitherT
import cats.syntax.flatMap._
import cats.syntax.functor._
import cats.effect.{Sync, Timer}
import cats.effect.concurrent.Ref
import fluence.effects.{Backoff, EffectError}
import fluence.effects.tendermint.block.TestData
import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.rpc.TendermintRpc
import fluence.effects.tendermint.rpc.http.{RpcError, RpcRequestFailed}
import fluence.effects.tendermint.rpc.websocket.{TestTendermintRpc, TestTendermintWebsocketRpc}
import fluence.log.Log

import scala.concurrent.duration._
import scala.language.higherKinds

class TendermintTest[F[_]: Timer: Monad](txRef: Ref[F, Either[RpcError, String]],
                                         consensusHeightRef: Ref[F, Either[RpcError, Long]],
                                         queryRef: Ref[F, Either[RpcError, String]]) {

  val tendermint: TendermintRpc[F] = new TestTendermintRpc[F] with TestTendermintWebsocketRpc[F] {
    override def subscribeNewBlock(lastKnownHeight: Long)(implicit log: Log[F],
                                                          backoff: Backoff[EffectError]): fs2.Stream[F, Block] =
      fs2.Stream
        .awakeEvery[F](500.milliseconds)
        .map(_ => Block(TestData.blockWithNullTxsResponse(1)).right.get)

    override def consensusHeight(id: String): EitherT[F, RpcError, Long] =
      EitherT(consensusHeightRef.get)

    override def broadcastTxSync(tx: String, id: String): EitherT[F, RpcError, String] =
      EitherT(txRef.get)

    override def query(
      path: String,
      data: String,
      height: Long,
      prove: Boolean,
      id: String
    ): EitherT[F, RpcError, String] = EitherT(queryRef.get)
  }

  def setTxResponse(response: Either[RpcError, String]): F[Unit] = txRef.set(response)
  def setQueryResponse(response: Either[RpcError, String]): F[Unit] = queryRef.set(response)
  def setConsensusHeightResponse(response: Either[RpcError, Long]): F[Unit] = consensusHeightRef.set(response)
}

object TendermintTest {

  def apply[F[_]: Monad: Timer: Sync](): F[TendermintTest[F]] = {
    val rpcError = Left(RpcRequestFailed(new RuntimeException("unimplemented"))): Either[RpcError, String]
    val rpcErrorC = Right(0): Either[RpcError, Long]
    for {
      txRef <- Ref.of(rpcError)
      queryRef <- Ref.of(rpcError)
      consensusRef <- Ref.of(rpcErrorC)
    } yield new TendermintTest[F](txRef, consensusRef, queryRef)
  }
}
