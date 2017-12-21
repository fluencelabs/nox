package fluence.dataset.protocol

import cats.{Applicative, ApplicativeError, Eq, MonadError}
import cats.syntax.applicative._
import fluence.dataset.peer.{ContractOps, ContractSignature}
import fluence.kad.Key

import scala.language.higherKinds

case class DumbContract(
    id: Key,
    size: Int,
    participants: Set[Key] = Set.empty,
    offerSealed: Boolean = false,
    participantsSealed: Boolean = false,
    version: Int = 0,
    allocationPossible: Boolean = true
)

object DumbContract {
  def signature(nodeId: Key): ContractSignature[DumbContract] = new ContractSignature[DumbContract] {
    /**
     * Add this node to list of contract's participants, with node's signature
     *
     * @param contract Contract offer
     * @return Updated contract with a signature
     */
    override def signOffer[F[_] : Applicative](contract: DumbContract): F[DumbContract] =
      contract.copy(participants = contract.participants + nodeId).pure[F]

    /**
     * Check that this is an offer, and current node has signed it
     *
     * @param contract Contract offer
     * @return Error or nothing
     */
    override def offerSigned[F[_]](contract: DumbContract)(implicit F: ApplicativeError[F, Throwable]): F[Unit] =
      if (contract.participants(nodeId)) F.pure(()) else F.raiseError(new IllegalArgumentException("Not signed"))

    /**
     * Check that this contract has list of participants, each participant signed it,
     * and list of participants is sealed by client.
     *
     * @param contract Contract with a list of participants
     * @return Error or nothing
     */
    override def participantsSealed[F[_]](contract: DumbContract)(implicit F: ApplicativeError[F, Throwable]): F[Unit] =
      if (contract.participantsSealed && contract.participants.size == contract.size) F.pure(()) else F.raiseError(new IllegalArgumentException("Not sealed"))

    /**
     * Check that this is an offer, and it's sealed by client
     *
     * @param contract Contract offer
     * @return Error or nothing
     */
    override def offerSealed[F[_]](contract: DumbContract)(implicit F: ApplicativeError[F, Throwable]): F[Unit] =
      if (contract.offerSealed) F.pure(()) else F.raiseError(new IllegalArgumentException("Not sealed"))
  }

  def ops(nodeId: Key): DumbContract ⇒ ContractOps[DumbContract] = c ⇒ new ContractOps[DumbContract](c, signature(nodeId)) {
    /**
     * Checks disk space availability,
     *
     * @return Nothing on success, failed F on error
     */
    override def checkAllocationPossible[F[_]](implicit F: MonadError[F, Throwable]): F[Unit] =
      if (c.allocationPossible) ().pure[F]
      else F.raiseError(new IllegalStateException("Can't allocate"))

    /**
     * Contract's version; used to check when a contract could be replaced with another one in cache.
     * Even if another contract is as cryptographically secure as current one, but is older, it should be rejected
     * to prevent replay attack on cache.
     *
     * @return Monotonic increasing contract version number
     */
    override def version: Long = c.version

    /**
     * Dataset ID
     *
     * @return Kademlia key of Dataset
     */
    override def id: Key = c.id

    /**
     * List of participating nodes Kademlia keys
     */
    override def participants: Set[Key] = c.participants
  }

  implicit val eq: Eq[DumbContract] = Eq.fromUniversalEquals
}
