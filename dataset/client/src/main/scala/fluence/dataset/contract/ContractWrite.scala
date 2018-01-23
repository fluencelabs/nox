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

import cats.MonadError
import cats.syntax.flatMap._
import fluence.crypto.signature.{ Signature, SignatureChecker, Signer }
import fluence.kad.protocol.Key

import scala.language.higherKinds

trait ContractWrite[C] {
  def setOfferSeal(contract: C, signature: Signature): C

  def setOfferSignature(contract: C, participant: Key, signature: Signature): C

  def setParticipantsSeal(contract: C, signature: Signature): C
}

object ContractWrite {
  implicit class WriteOps[C](contract: C)(implicit read: ContractRead[C], write: ContractWrite[C]) {
    import ContractRead.ReadOps

    def sealOffer(signer: Signer): C =
      write.setOfferSeal(contract, signer.sign(contract.getOfferBytes))

    def signOffer(participant: Key, signer: Signer): C =
      write.setOfferSignature(contract, participant, signer.sign(contract.getOfferBytes))

    def sealParticipants(signer: Signer): C =
      write.setParticipantsSeal(contract, signer.sign(contract.getParticipantsBytes))

    def addParticipants[F[_]](checker: SignatureChecker, participants: Seq[C])(implicit F: MonadError[F, Throwable]): F[C] =
      F.catchNonFatal(participants.foldLeft(contract) {
        case (agg, part) if part.participants.size == 1 ⇒
          part.participants.headOption
            .flatMap(p ⇒ part.participantSignature(p).map(p -> _))
            .fold(agg){
              case (p, sign) ⇒ write.setOfferSignature(agg, p, sign)
            }

        case (agg, _) ⇒
          agg
      }).flatMap {
        case signed if signed.checkAllParticipants(checker) ⇒
          F.pure(signed)

        case _ ⇒
          F.raiseError(new IllegalArgumentException("Wrong number of participants or wrong signatures"))
      }
  }
}
