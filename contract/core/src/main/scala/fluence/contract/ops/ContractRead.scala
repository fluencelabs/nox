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

package fluence.contract.ops

import cats.data.EitherT
import cats.Monad
import fluence.crypto.SignAlgo.CheckerFn
import fluence.crypto.algorithm.CryptoErr
import fluence.crypto.keypair.KeyPair
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
   * Public key of this contract owner
   */
  def publicKey(contract: C): KeyPair.Public

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
   */
  def getOfferBytes(contract: C): ByteVector

  /**
   * Returns client's signature for offer bytes
   */
  def offerSeal(contract: C): Signature

  /**
   * Returns participants bytes representation to be sealed by client
   */
  def getParticipantsBytes(contract: C): ByteVector

  /**
   * Returns client's signature for participants list, if it's already sealed
   */
  def participantsSeal(contract: C): Option[Signature]

  /**
   * Returns contract execution state's bytes representation, used to sign & verify signatures
   */
  def getExecutionStateBytes(contract: C): ByteVector

  /**
   * Returns client's signature for execution state bytes
   */
  def executionStateSeal(contract: C): Signature

}

object ContractRead {

  implicit class ReadOps[C](contract: C)(implicit read: ContractRead[C]) {

    def id: Key = read.id(contract)

    def publicKey: KeyPair.Public = read.publicKey(contract)

    def version: Long = read.version(contract)

    def participants: Set[Key] = read.participants(contract)

    def participantsRequired: Int = read.participantsRequired(contract)

    def participantSignature(participant: Key): Option[Signature] = read.participantSignature(contract, participant)

    def getOfferBytes: ByteVector = read.getOfferBytes(contract)

    def offerSeal: Signature = read.offerSeal(contract)

    def getParticipantsBytes: ByteVector = read.getParticipantsBytes(contract)

    def participantsSeal: Option[Signature] = read.participantsSeal(contract)

    def getExecutionStateBytes: ByteVector = read.getExecutionStateBytes(contract)

    def executionStateSeal: Signature = read.executionStateSeal(contract)

    /**
     * Checks that client's seal for the contract offer is correct
     *
     * @param checkerFn Creates checker for specified public key
     */
    def checkOfferSeal[F[_]: Monad]()(implicit checkerFn: CheckerFn): EitherT[F, CryptoErr, Unit] =
      for {
        _ ← checkOfferSignature(offerSeal, checkerFn(publicKey))
        _ ← checkPubKey
      } yield ()

    /**
     * Checks that signature matches contract's offer
     *
     * @param signature Signature to check
     * @param checker Signature checker
     */
    private def checkOfferSignature[F[_]: Monad](
      signature: Signature,
      checker: SignatureChecker
    ): EitherT[F, CryptoErr, Unit] =
      checker.check[F](signature.sign, getOfferBytes)
        .leftMap(e ⇒ e.copy(errorMessage = s"Offer seal is not verified for contract(id=$id)"))

    /**
     * @return Whether this contract is a valid blank offer (with no participants, with client's signature)
     */
    def isBlankOffer[F[_]: Monad]()(implicit checkerFn: CheckerFn): EitherT[F, CryptoErr, Boolean] =
      if (participants.isEmpty)
        checkOfferSeal().map(_ ⇒ true)
      else
        EitherT.rightT(false)

    /**
     * Checks that participant has signed an offer
     *
     * @param participant Participating node's key
     * @param checkerFn Creates checker for specified public key
     * @return ''true'' if participant has signed an offer, ''false'' otherwise or ''CryptoError'' is something go wrong
     */
    def participantSigned[F[_]: Monad](
      participant: Key
    )(implicit checkerFn: CheckerFn): EitherT[F, CryptoErr, Boolean] =
      participantSignature(participant).map { pSignature ⇒
        val participantChecker = checkerFn(pSignature.publicKey)
        for {
          _ ← checkOfferSignature(pSignature, participantChecker)
          _ ← checkPubKey
        } yield true
      }.getOrElse(EitherT.rightT(false))

    /**
     * Checks that seal for the contracts participants is correct if it present.
     *
     * @param checkerFn Creates checker for specified public keyr
     */
    def checkParticipantsSeal[F[_]: Monad]()(
      implicit checkerFn: CheckerFn
    ): EitherT[F, CryptoErr, Option[Unit]] =
      participantsSeal match {
        case Some(sign) ⇒
          for {
            _ ← checkerFn(publicKey).check[F](sign.sign, getParticipantsBytes)
                .leftMap(e ⇒ e.copy(errorMessage = s"Participants seal is not verified for contract(id=$id)"))
            _ ← checkPubKey
          } yield Some(())
        case None ⇒
          EitherT.rightT[F, CryptoErr](None)
      }

    /**
     * Checks that number of participants is correct, and all signatures are valid.
     *
     * @param checkerFn Creates checker for specified public key
     * @return Unit if signatures of all required participants is valid, raise error otherwise
     */
    def checkAllParticipants[F[_]: Monad]()(implicit checkerFn: CheckerFn): EitherT[F, CryptoErr, Unit] =
      if (participants.size == participantsRequired) {
        type M[A] = EitherT[F, CryptoErr, A]
        Monad[M].tailRecM(participants.toStream) {
          case pk #:: tail ⇒
            participantSigned[F](pk)
              .map[Either[Stream[Key], Unit]] {
                case true ⇒ Left(tail)
                case false ⇒ Right(false)
              }
          case _ ⇒
            Monad[M].pure(Right[Stream[Key], Unit](()))
        }
      } else EitherT.leftT(CryptoErr("Wrong number of participants"))

    /**
     * Checks that client's seal for the contract execution state is correct
     *
     * @param checkerFn Creates checker for specified public key
     */
    def checkExecStateSeal[F[_]: Monad]()(implicit checkerFn: CheckerFn): EitherT[F, CryptoErr, Unit] =
      for {
        _ ← checkExecStateSignature(executionStateSeal)
        _ ← checkPubKey
      } yield ()

    /**
     * Checks that signature matches contract's execution state
     *
     * @param signature Signature to check
     * @param checkerFn Creates checker for specified public key
     */
    private def checkExecStateSignature[F[_]: Monad](
      signature: Signature
    )(implicit checkerFn: CheckerFn): EitherT[F, CryptoErr, Unit] =
      checkerFn(publicKey).check[F](signature.sign, getExecutionStateBytes)
        .leftMap(e ⇒ e.copy(errorMessage = s"Execution state seal is not verified for contract(id=$id)"))

    /**
     * @return Whether this contract is successfully signed by all participants, and participants list is sealed by client
     */
    def isActiveContract[F[_]: Monad]()(implicit checkerFn: CheckerFn): EitherT[F, CryptoErr, Boolean] =
      for {
        _ ← checkOfferSeal()
        _ ← checkExecStateSeal()
        _ ← checkAllParticipants()
        participantSealResult ← checkParticipantsSeal
      } yield participantSealResult.isDefined

    /**
     * Checks all seals sealed by contract owner. Note that this method don't check participants signatures.
     * @return Right(unit) if all owners seals is correct, Left(error) otherwise.
     */
    def checkAllOwnerSeals[F[_]: Monad]()(implicit checkerFn: CheckerFn): EitherT[F, CryptoErr, Unit] =
      for {
        _ ← contract.checkOfferSeal()
        _ ← contract.checkParticipantsSeal()
        _ ← contract.checkExecStateSeal()
      } yield ()

    /**
     * Checks that given key is produced form that publicKey
     */
    def checkPubKey[F[_]: Monad]: EitherT[F, CryptoErr, Unit] =
      EitherT.cond(
        Key.checkPublicKey(id, publicKey),
        (),
        CryptoErr(s"Contract id is not equals to hash(pubKey); id=$id pubKey=$publicKey")
      )

  }

}
