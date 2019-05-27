package fluence.effects.receipt.storage

import scala.language.higherKinds

/**
 * Algebra for storage of block receipts
 *
 * @tparam AppID Application ID type, could be just Long
 * @tparam Receipt Receipt type
 */
trait ReceiptStorage[AppID, Receipt] {

  /**
   * Stores receipt for the specified app at a given height
   *
   * @param id Application id, to identify which app receipt belongs to
   * @param height Block height for the receipt
   * @param receipt Block receipt
   */
  def put[F[_]](id: AppID, height: Long, receipt: Receipt): F[Unit]

  /**
   * Gets a receipt for specified app and height
   *
   * @param id Application id
   * @param height Block height
   * @return Receipt if exists
   */
  def get[F[_]](id: AppID, height: Long): F[Option[Receipt]]

  /**
   * Retrieves a chain of receipts, starting at block height `from`, until `to`
   *
   * @param id Application id
   * @param from Height of the first receipt in the resulting chain
   * @param to Height of the last receipt in the resulting chain
   * @return Chain of receipts
   */
  def retrieve[F[_]](id: AppID, from: Option[Long] = None, to: Option[Long] = None): F[List[Receipt]]
}
