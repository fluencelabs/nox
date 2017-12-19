package fluence.dataset.allocate

import cats.{ Applicative, ApplicativeError }

import scala.language.higherKinds

/**
 * Performs signature checks & writes for contract
 * @tparam C contract
 */
trait ContractSignature[C] {
  /**
   * Check that this is an offer, and it's sealed by client
   * @param contract Contract offer
   * @return Error or nothing
   */
  def offerSealed[F[_]](contract: C)(implicit F: ApplicativeError[F, Throwable]): F[Unit]

  /**
   * Check that this is an offer, and current node has signed it
   * @param contract Contract offer
   * @return Error or nothing
   */
  def offerSigned[F[_]](contract: C)(implicit F: ApplicativeError[F, Throwable]): F[Unit]

  /**
   * Add this node to list of contract's participants, with node's signature
   * @param contract Contract offer
   * @return Updated contract with a signature
   */
  def signOffer[F[_] : Applicative](contract: C): F[C]

  /**
   * Check that this contract has list of participants, each participant signed it,
   * and list of participants is sealed by client.
   * @param contract Contract with a list of participants
   * @return Error or nothing
   */
  def participantsSealed[F[_]](contract: C)(implicit F: ApplicativeError[F, Throwable]): F[Unit]
}
