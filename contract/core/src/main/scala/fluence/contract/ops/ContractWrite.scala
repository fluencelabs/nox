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
import cats.{Invariant, Monad}
import fluence.crypto.signature.SignAlgo.CheckerFn
import fluence.crypto.CryptoError
import fluence.crypto.signature.{PubKeyAndSignature, Signature, Signer}
import fluence.kad.protocol.Key

import scala.language.higherKinds

trait ContractWrite[C] {

  def setOfferSeal(contract: C, signature: Signature): C

  def setOfferSignature(contract: C, participant: Key, keyAndSign: PubKeyAndSignature): C

  def setParticipantsSeal(contract: C, signature: Signature): C

  def setExecStateSeal(contract: C, signature: Signature): C

}

object ContractWrite {

  implicit object inv extends Invariant[ContractWrite] {
    override def imap[A, B](fa: ContractWrite[A])(f: A ⇒ B)(g: B ⇒ A): ContractWrite[B] =
      new ContractWrite[B] {
        override def setOfferSeal(contract: B, signature: Signature): B =
          f(fa.setOfferSeal(g(contract), signature))

        override def setOfferSignature(contract: B, participant: Key, keyAndSign: PubKeyAndSignature): B =
          f(fa.setOfferSignature(g(contract), participant, keyAndSign))

        override def setParticipantsSeal(contract: B, signature: Signature): B =
          f(fa.setParticipantsSeal(g(contract), signature))

        override def setExecStateSeal(contract: B, signature: Signature): B =
          f(fa.setExecStateSeal(g(contract), signature))
      }
  }

  implicit class WriteOps[F[_]: Monad, C](contract: C)(
    implicit read: ContractRead[C],
    write: ContractWrite[C]
  ) {
    import ContractRead.ReadOps

    /**
     * Seals contract offer.
     *
     * @param signer Algorithm to produce signatures for this participant
     */
    def sealOffer(signer: Signer): EitherT[F, CryptoError, C] =
      signer
        .sign(contract.getOfferBytes)
        .map(s ⇒ write.setOfferSeal(contract, s))

    /**
     * Signs contracts offer with the specified key and add this signature as participant into contract.
     *
     * @param participant Participants private key
     * @param signer Algorithm to produce signatures for this participant
     */
    def signOffer(participant: Key, signer: Signer): EitherT[F, CryptoError, C] =
      signer
        .sign(contract.getOfferBytes)
        .map(s ⇒ write.setOfferSignature(contract, participant, PubKeyAndSignature(signer.publicKey, s)))

    /**
     * Seals participants into contract.
     *
     * @param signer Algorithm to produce signatures for this participant
     */
    def sealParticipants(signer: Signer): EitherT[F, CryptoError, C] =
      signer
        .sign(contract.getParticipantsBytes)
        .map(s ⇒ write.setParticipantsSeal(contract, s))

    /**
     * Adds specified contracts (signed by participants) as participants to base contract.
     *
     * @param participants Contracts signed by participants
     * @param checkerFn Creates checker for specified public key
     * @return Contract with filled participants signatures
     */
    def addParticipants(participants: Seq[C])(implicit checkerFn: CheckerFn): EitherT[F, CryptoError, C] =
      EitherT
        .rightT(participants.foldLeft(contract) {
          case (agg, part) if part.participants.size == 1 ⇒
            part.participants.headOption
              .flatMap(p ⇒ part.participantSignature(p).map(p -> _))
              .fold(agg) {
                case (participantKey, participantSign) ⇒ write.setOfferSignature(agg, participantKey, participantSign)
              }

          case (agg, _) ⇒
            agg
        })
        .flatMap { signed ⇒
          signed.checkAllParticipants().map(_ ⇒ signed)
        }

    /**
     * Seals contract execution state.
     *
     * @param signer Algorithm to produce signatures for this participant
     */
    def sealExecState(signer: Signer): EitherT[F, CryptoError, C] =
      signer
        .sign(contract.getExecutionStateBytes)
        .map(s ⇒ write.setExecStateSeal(contract, s))
  }
}
