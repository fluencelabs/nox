package fluence.bp.tendermint

import cats.data.EitherT
import fluence.bp.api.BlockProducer
import fluence.bp.tx.TxResponse
import fluence.effects.EffectError
import fluence.log.Log

import scala.language.higherKinds

class TendermintBlockProducer[F[_]] extends BlockProducer[F] {
  // TODO put the real Tendermint block here
  override type Block = this.type

  /**
   * Retrieve the last height, known locally
   * TODO read it from BlockStore?
   */
  override def lastKnownHeight()(implicit log: Log[F]): F[Long] = ???

  /**
   * Stream of blocks, starting with the given height
   * TODO get it from BlockStore, then switch to websocket?
   *
   * @param fromHeight All newer blocks shall appear in the stream
   * @return Stream of blocks
   */
  override def blockStream(fromHeight: Long)(implicit log: Log[F]): fs2.Stream[F, Block] = ???

  /**
   * Send (asynchronously) a transaction to the block producer, so that it should later get into a block
   * TODO really sendTx via Tendermint RPC
   *
   * @param txData Transaction data
   */
  override def sendTx(txData: Array[Byte])(implicit log: Log[F]): EitherT[F, EffectError, TxResponse] = ???
}
