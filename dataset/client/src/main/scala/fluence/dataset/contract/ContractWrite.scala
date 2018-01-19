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
