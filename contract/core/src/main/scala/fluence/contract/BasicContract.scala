/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.contract

import java.nio.ByteBuffer

import cats.syntax.flatMap._
import cats.{Eq, MonadError}
import fluence.contract.BasicContract.ExecutionState
import fluence.contract.ops.{ContractRead, ContractWrite}
import fluence.crypto.signature.{Signature, Signer}
import fluence.kad.protocol.Key
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * TODO: shouldn't it be in protocol?
 *
 * @param id               Contract/cluster ID
 * @param offer            The offer from client
 * @param offerSeal        Client's signature for contract offer
 * @param participants     Map of participant node keys to participant signatures, sorted by distance from id
 * @param participantsSeal Client's signature for a list of participants
 * @param executionState          State of the contract that changes over time, e.g. when merkle root changes
 * @param executionSeal Client's signature for executionState
 */
case class BasicContract(
  id: Key,
  offer: BasicContract.Offer,
  offerSeal: Signature,
  participants: Map[Key, Signature],
  participantsSeal: Option[Signature],
  executionState: ExecutionState,
  executionSeal: Signature
)

object BasicContract {

  case class Offer(participantsRequired: Int) {
    lazy val getBytes: ByteVector = ByteVector.fromInt(participantsRequired)
  }

  case class ExecutionState(version: Long, merkleRoot: ByteVector) {
    lazy val getBytes: ByteVector = ByteVector.fromLong(version) ++ merkleRoot
  }

  // TODO: EitherT instead of MonadError
  def offer[F[_]](id: Key, participantsRequired: Int, signer: Signer)(
    implicit F: MonadError[F, Throwable]
  ): F[BasicContract] = {
    val offer = Offer(participantsRequired)
    val execState = ExecutionState(0, ByteVector.empty)

    val newContract = for {
      offerSeal ← signer.sign(offer.getBytes)
      execStateSeal ← signer.sign(execState.getBytes)
    } yield {
      BasicContract(id, offer, offerSeal, Map.empty, None, execState, execStateSeal)
    }

    newContract.value.flatMap(F.fromEither)
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
    override def getOfferBytes(contract: BasicContract): ByteVector =
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
