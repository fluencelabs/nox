package fluence.effects.receipt.storage

import cats.data.EitherT
import fluence.effects.tendermint.block.history.Receipt
import fluence.log.Log

import scala.language.higherKinds

/**
 * Algebra for storage of block receipts.
 */
trait ReceiptStorage[F[_]] {
  val appId: Long

  /**
   * Stores receipt for the specified app at a given height.
   *
   * @param height block height for the receipt
   * @param receipt block receipt
   */
  def put(height: Long, receipt: Receipt)(implicit log: Log[F]): EitherT[F, ReceiptStorageError, Unit]

  /**
   * Gets a receipt for specified app and height
   *
   * @param height block height
   * @return receipt if exists
   */
  def get(height: Long)(implicit log: Log[F]): EitherT[F, ReceiptStorageError, Option[Receipt]]

  /**
   * Retrieves a chain of receipts, starting at block height `from`, until `to`.
   *
   * @param from height of the first receipt in the resulting chain
   * @param to height of the last receipt in the resulting chain
   * @return chain of receipts
   */
  def retrieve(
    from: Option[Long] = None,
    to: Option[Long] = None
  )(implicit log: Log[F]): fs2.Stream[F, (Long, Receipt)]
}
