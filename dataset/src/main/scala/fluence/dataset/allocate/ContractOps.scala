package fluence.dataset.allocate

import cats.{ Id, Monad, MonadError }
import cats.instances.try_._
import cats.syntax.flatMap._
import cats.syntax.functor._
import fluence.kad.Key

import scala.language.higherKinds
import scala.util.Try

/**
 * Common operations for contract allocation and caching
 *
 * @param contract The contract
 * @param contractSignature Signature checker & signer service
 * @tparam C Contract's type
 */
abstract class ContractOps[C](contract: C, contractSignature: ContractSignature[C]) {

  /**
   * Dataset ID
   *
   * @return Kademlia key of Dataset
   */
  def id: Key

  /**
   * Contract's version; used to check when a contract could be replaced with another one in cache.
   * Even if another contract is as cryptographically secure as current one, but is older, it should be rejected
   * to prevent replay attack on cache.
   *
   * @return Monotonic increasing contract version number
   */
  def version: Long

  /**
   * List of participating nodes Kademlia keys
   */
  def participants: Set[Key]

  /**
   * Checks disk space availability
   *
   * @return Nothing on success, failed F on error
   */
  def checkAllocationPossible[F[_]](implicit F: MonadError[F, Throwable]): F[Unit]

  /**
   * @return true if current node participates in this contract
   */
  def nodeParticipates: Boolean =
    contractSignature.offerSigned[Try](contract).isSuccess

  /**
   * @return Whether this contract is a valid blank offer (with no participants, with client's signature)
   */
  def isBlankOffer: Boolean =
    participants.isEmpty && contractSignature.offerSealed[Try](contract).isSuccess && version == 0

  /**
   * @return Whether this contract offer was signed by this node and client, but participants list is not sealed yet
   */
  def isSignedOffer: Boolean =
    participants.size == 1 &&
      contractSignature.offerSealed[Try](contract).isSuccess &&
      nodeParticipates &&
      version == 0 &&
      contractSignature.participantsSealed[Try](contract).isFailure

  /**
   * @return Whether this contract is successfully signed by all participants, and participants list is sealed by client
   */
  def isActiveContract: Boolean =
    contractSignature.participantsSealed[Try](contract).isSuccess

  /**
   * Sign a blank offer by current node
   *
   * @return Signed offer
   */
  def signOffer: C =
    contractSignature.signOffer[Id](contract)

  /**
   * Convert a contract to a record to be stored in local cache
   */
  private[dataset] def record: ContractRecord[C] =
    ContractRecord(contract)

  /**
   * @return Whether this node can be cached
   */
  def canBeCached: Boolean =
    !nodeParticipates && isActiveContract

  /**
   * Performs signature checks for this node's signature, and client's seal on participants list
   *
   * @return Nothing on success, failure on error
   */
  def withSealedOffer[F[_]](implicit ME: MonadError[F, Throwable]): F[Unit] =
    for {
      _ ← contractSignature.offerSigned[F](contract)
      _ ← contractSignature.participantsSealed[F](contract)
    } yield ()
}
