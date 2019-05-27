package fluence.effects.receipt.storage

import cats.data.EitherT
import fluence.effects.tendermint.block.history.Receipt

import scala.language.higherKinds

/**
 * Algebra for storage of block receipts
 *
 */
trait ReceiptStorage[F[_]] {

  /**
   * Stores receipt for the specified app at a given height
   *
   * @param appId Application id, to identify which app receipt belongs to
   * @param height Block height for the receipt
   * @param receipt Block receipt
   */
  def put(appId: Long, height: Long, receipt: Receipt): EitherT[F, ReceiptStorageError, Unit]

  /**
   * Gets a receipt for specified app and height
   *
   * @param appId Application id
   * @param height Block height
   * @return Receipt if exists
   */
  def get(appId: Long, height: Long): EitherT[F, ReceiptStorageError, Option[Receipt]]

  /**
   * Retrieves a chain of receipts, starting at block height `from`, until `to`
   *
   * @param appId Application id
   * @param from Height of the first receipt in the resulting chain
   * @param to Height of the last receipt in the resulting chain
   * @return Chain of receipts
   */
  def retrieve(
    appId: Long,
    from: Option[Long] = None,
    to: Option[Long] = None
  ): F[List[(Long, Receipt)]]
}
