package fluence.dataset

import java.nio.ByteBuffer

import cats.Eq
import fluence.crypto.signature.{ Signature, Signer }
import fluence.dataset.contract.{ ContractRead, ContractWrite }
import fluence.kad.protocol.Key

/**
 *
 * @param id               Contract/cluster ID
 * @param offer            The offer from client
 * @param offerSeal        Client's signature for contract offer
 * @param participants     Map of participant node keys to participant signatures, sorted by distance from id
 * @param participantsSeal Client's signature for a list of participants
 * @param version          Incremented over the lifecycle of contract
 */
case class BasicContract(
    id: Key,

    offer: BasicContract.Offer,

    offerSeal: Signature,

    participants: Map[Key, Signature],

    participantsSeal: Option[Signature],

    version: Long
)

object BasicContract {

  case class Offer(participantsRequired: Int) {
    lazy val getBytes: Array[Byte] = {
      val buffer = ByteBuffer.allocate(java.lang.Integer.BYTES)

      buffer.putInt(participantsRequired)

      buffer.array()
    }
  }

  def offer(id: Key, participantsRequired: Int, signer: Signer): BasicContract =
    {
      val offer = Offer(participantsRequired)
      val signature = signer.sign(offer.getBytes)
      BasicContract(id, offer, signature, Map.empty, None, 0)
    }

  // TODO: there should be contract laws, like "init empty - not signed -- sign offer -- signed, no participants -- add participant -- ..."

  implicit object BasicContractWrite extends ContractWrite[BasicContract] {
    override def setOfferSeal(contract: BasicContract, signature: Signature): BasicContract =
      contract.copy(offerSeal = signature)

    override def setOfferSignature(contract: BasicContract, participant: Key, signature: Signature): BasicContract =
      contract.copy(participants = contract.participants + (participant -> signature))

    override def setParticipantsSeal(contract: BasicContract, signature: Signature): BasicContract =
      contract.copy(participantsSeal = Some(signature))
  }

  implicit object BasicContractRead extends ContractRead[BasicContract] {
    /**
     * Dataset ID
     *
     * @return Kademlia key of Dataset
     */
    override def id(contract: BasicContract): Key =
      contract.id

    /**
     * Contract's version; used to check when a contract could be replaced with another one in cache.
     * Even if another contract is as cryptographically secure as current one, but is older, it should be rejected
     * to prevent replay attack on cache.
     *
     * @return Monotonic increasing contract version number
     */
    override def version(contract: BasicContract): Long =
      0 // TODO: version updates must be signed somehow in order to avoid unauthorized changes; unless it's done, versioning is not supported

    /**
     * List of participating nodes Kademlia keys
     */
    override def participants(contract: BasicContract): Set[Key] =
      contract.participants.keySet

    /**
     * How many participants (=replicas) is required for the contract
     */
    override def participantsRequired(contract: BasicContract): Int =
      contract.offer.participantsRequired

    /**
     * Participant's signature for an offer, if any
     *
     * @param contract    Contract
     * @param participant Participating node's key
     */
    override def participantSignature(contract: BasicContract, participant: Key): Option[Signature] =
      contract.participants.get(participant)

    /**
     * Returns contract offer's bytes representation, used to sign & verify signatures
     *
     * @param contract Contract
     */
    override def getOfferBytes(contract: BasicContract): Array[Byte] =
      contract.offer.getBytes

    /**
     * Returns client's signature for offer bytes
     *
     * @param contract Contract
     */
    override def offerSeal(contract: BasicContract): Signature =
      contract.offerSeal

    /**
     * Returns client's signature for participants list, if it's already sealed
     *
     * @param contract Contract
     */
    override def participantsSeal(contract: BasicContract): Option[Signature] =
      contract.participantsSeal
  }

  implicit val eq: Eq[BasicContract] = Eq.fromUniversalEquals

}
