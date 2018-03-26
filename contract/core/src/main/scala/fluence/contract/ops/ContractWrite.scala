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
import fluence.crypto.algorithm.CryptoErr
import fluence.crypto.signature.{Signature, SignatureChecker, Signer}
import fluence.kad.protocol.Key

import scala.language.higherKinds

trait ContractWrite[C] {

  def setOfferSeal(contract: C, signature: Signature): C

  def setOfferSignature(contract: C, participant: Key, signature: Signature): C

  def setParticipantsSeal(contract: C, signature: Signature): C

}

object ContractWrite {

  implicit object inv extends Invariant[ContractWrite] {
    override def imap[A, B](fa: ContractWrite[A])(f: A ⇒ B)(g: B ⇒ A): ContractWrite[B] =
      new ContractWrite[B] {
        override def setOfferSeal(contract: B, signature: Signature): B =
          f(fa.setOfferSeal(g(contract), signature))

        override def setOfferSignature(contract: B, participant: Key, signature: Signature): B =
          f(fa.setOfferSignature(g(contract), participant, signature))

        override def setParticipantsSeal(contract: B, signature: Signature): B =
          f(fa.setParticipantsSeal(g(contract), signature))
      }
  }

  implicit class WriteOps[F[_]: Monad, C](contract: C)(
    implicit read: ContractRead[C],
    write: ContractWrite[C]
  ) {
    import ContractRead.ReadOps

    def sealOffer(signer: Signer): EitherT[F, CryptoErr, C] =
      signer
        .sign(contract.getOfferBytes)
        .map(s ⇒ write.setOfferSeal(contract, s))

    /**
     * Sign the contract with the specified key.
     *
     * @param participant Participants private key
     * @param signer Algorithm to produce signatures for this participant
     */
    def signOffer(participant: Key, signer: Signer): EitherT[F, CryptoErr, C] =
      signer
        .sign(contract.getOfferBytes)
        .map(s ⇒ write.setOfferSignature(contract, participant, s))

    /**
     * Seals participants into contract.
     *
     * @param signer Algorithm to produce signatures for this participant
     */
    def sealParticipants(signer: Signer): EitherT[F, CryptoErr, C] =
      signer
        .sign(contract.getParticipantsBytes)
        .map(s ⇒ write.setParticipantsSeal(contract, s))

    /**
     * Add contracts signed by participants to base contract.
     *
     * @param participants Contracts signed by participants
     * @param checker Algorithm to produce signatures for this participant
     * @return Contract with filled participants signatures
     */
    def addParticipants(participants: Seq[C])(implicit checker: SignatureChecker): EitherT[F, CryptoErr, C] =
      EitherT
        .rightT(participants.foldLeft(contract) {
          case (agg, part) if part.participants.size == 1 ⇒
            part.participants.headOption
              .flatMap(p ⇒ part.participantSignature(p).map(p -> _))
              .fold(agg) {
                case (p, sign) ⇒ write.setOfferSignature(agg, p, sign)
              }

          case (agg, _) ⇒
            agg
        })
        .flatMap { signed ⇒
          signed.checkAllParticipants().map(_ ⇒ signed)
        }
  }
}
