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

package fluence.dataset.contract

import cats.syntax.functor._
import cats.syntax.flatMap._
import cats.instances.list._
import cats.syntax.applicative._
import cats.{ MonadError, Traverse }
import fluence.crypto.signature.{ Signature, SignatureChecker }
import fluence.kad.protocol.Key
import scodec.bits.ByteVector

import scala.language.higherKinds

/**
 * Abstracts out read operations for the contract
 * @tparam C Contract's type
 */
trait ContractRead[C] {
  /**
   * Cluster ID
   *
   * @return Kademlia key of Dataset
   */
  def id(contract: C): Key

  /**
   * Contract's version; used to check when a contract could be replaced with another one in cache.
   * Even if another contract is as cryptographically secure as current one, but is older, it should be rejected
   * to prevent replay attack on cache.
   *
   * @return Monotonic increasing contract version number
   */
  def version(contract: C): Long

  /**
   * List of participating nodes Kademlia keys
   */
  def participants(contract: C): Set[Key]

  /**
   * How many participants (=replicas) is required for the contract
   */
  def participantsRequired(contract: C): Int

  /**
   * Participant's signature for an offer, if any
   *
   * @param contract Contract
   * @param participant Participating node's key
   */
  def participantSignature(contract: C, participant: Key): Option[Signature]

  /**
   * Returns contract offer's bytes representation, used to sign & verify signatures
   *
   * @param contract Contract
   */
  def getOfferBytes(contract: C): ByteVector

  /**
   * Returns client's signature for offer bytes
   *
   * @param contract Contract
   */
  def offerSeal(contract: C): Signature

  /**
   * Returns client's signature for participants list, if it's already sealed
   *
   * @param contract Contract
   */
  def participantsSeal(contract: C): Option[Signature]
}

object ContractRead {

  implicit class ReadOps[C](contract: C)(implicit read: ContractRead[C]) {

    def id: Key = read.id(contract)

    def version: Long = read.version(contract)

    def participants: Set[Key] = read.participants(contract)

    def participantsRequired: Int = read.participantsRequired(contract)

    def participantSignature(participant: Key): Option[Signature] = read.participantSignature(contract, participant)

    def getOfferBytes: ByteVector = read.getOfferBytes(contract)

    def offerSeal: Signature = read.offerSeal(contract)

    def participantsSeal: Option[Signature] = read.participantsSeal(contract)

    /**
     * Returns participants bytes representation to be sealed by client
     */
    def getParticipantsBytes: ByteVector =
      // TODO: review, document & optimize
      participants
        .toSeq
        .sorted(Key.relativeOrdering(id))
        .map(k ⇒
          participantSignature(k).fold(ByteVector.empty)(_.sign)
        ).foldLeft(id.value)(_ ++ _)

    /**
     * Checks that client's seal for the contract offer is correct
     *
     * @param checker Signature checker
     */
    def checkOfferSeal[F[_]](checker: SignatureChecker[F])(implicit F: MonadError[F, Throwable]): F[Boolean] = {
      for {
        checkSign ← checkOfferSignature(offerSeal, checker)
      } yield Key.checkPublicKey(id, offerSeal.publicKey) && checkSign
    }

    /**
     * Checks that signature matches contract's offer
     *
     * @param signature Signature to check
     * @param checker Signature checker
     */
    def checkOfferSignature[F[_]](signature: Signature, checker: SignatureChecker[F])(implicit F: MonadError[F, Throwable]): F[Boolean] =
      checker.check(signature, getOfferBytes)

    /**
     * @return Whether this contract is a valid blank offer (with no participants, with client's signature)
     */
    def isBlankOffer[F[_]](checker: SignatureChecker[F])(implicit F: MonadError[F, Throwable]): F[Boolean] =
      if (participants.isEmpty) checkOfferSeal(checker) else false.pure[F]

    /**
     * @return Whether this contract offer was signed by a single node and client, but participants list is not sealed yet
     */
    def isSignedParticipant[F[_]](checker: SignatureChecker[F])(implicit F: MonadError[F, Throwable]): F[Boolean] =
      participants.toList match {
        case single :: Nil ⇒
          participantSigned(single, checker)

        case _ ⇒
          F.pure(false)
      }

    /**
     * Checks that participant has signed an offer
     *
     * @param participant Participating node's key
     * @param checker Signature checker
     */
    def participantSigned[F[_]](participant: Key, checker: SignatureChecker[F])(implicit F: MonadError[F, Throwable]): F[Boolean] = {
      participantSignature(participant) match {
        case Some(ps) ⇒ checkOfferSignature(ps, checker).map(_ ⇒ Key.checkPublicKey(participant, ps.publicKey))
        case None     ⇒ F.pure(false)
      }
    }

    /**
     * Checks that number of participants is correct, and all signatures are valid
     *
     * @param checker Signature checker
     */
    def checkAllParticipants[F[_]](checker: SignatureChecker[F])(implicit F: MonadError[F, Throwable]): F[Boolean] = {
      if (participants.size == participantsRequired)
        Traverse[List].traverse(participants.map(participantSigned(_, checker)).toList)(identity).map(_.forall(identity))
      else false.pure[F]
    }

    /**
     * @return Whether this contract is successfully signed by all participants, and participants list is sealed by client
     */
    def isActiveContract[F[_]](checker: SignatureChecker[F])(implicit F: MonadError[F, Throwable]): F[Boolean] = {
      for {
        offerSealResult ← checkOfferSeal(checker)
        participants ← Traverse[List].traverse(participants.map(participantSigned(_, checker)).toList)(identity)
        participantSealResult ← participantsSeal.filter(seal ⇒ Key.checkPublicKey(id, seal.publicKey)) match {
          case Some(sign) ⇒ checker.check(sign, getParticipantsBytes)
          case None       ⇒ F.pure(false)
        }
      } yield offerSealResult && participants.forall(identity) && participantSealResult
    }
  }

}
