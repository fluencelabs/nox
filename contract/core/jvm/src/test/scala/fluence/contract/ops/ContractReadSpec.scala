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

import cats.Now
import cats.instances.option._
import cats.instances.try_._
import fluence.contract.{BasicContract, _}
import fluence.crypto.algorithm.{CryptoErr, Ecdsa}
import fluence.crypto.signature.{Signature, SignatureChecker}
import fluence.kad.protocol.Key
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.util.Try

class ContractReadSpec extends WordSpec with Matchers {

  // contract constants
  private val signAlgo = Ecdsa.signAlgo
  private val contractOwnerKeyPair = signAlgo.generateKeyPair[Option]().success
  private val contractOwnerSigner = signAlgo.signer(contractOwnerKeyPair)
  private val contractKadKey = Key.fromKeyPair[Try](contractOwnerKeyPair).get

  import ContractRead.ReadOps
  import ContractWrite.WriteOps
  import signAlgo.checker

  private val contract: BasicContract = BasicContract.offer(contractKadKey, 2, contractOwnerSigner).get
  private val corruptedSignature: Signature =
    contractOwnerSigner.sign[Option](ByteVector("corruption".getBytes)).success
  private val maliciousPubKey = signAlgo.generateKeyPair[Option]().success.publicKey

  private def corruptSignature(signature: Signature): Signature =
    signature.copy(sign = corruptedSignature.sign)

  private def changeToMaliciousPubKey(signature: Signature): Signature =
    signature.copy(publicKey = maliciousPubKey)

  "ContractRead.ReadOps.checkOfferSeal" should {
    "fail" when {
      "offer seal is corrupted" in {
        val result =
          sealOffer(contract)
            .copy(offerSeal = corruptSignature(contract.offerSeal))
            .checkOfferSeal[Option]

        result.failed shouldBe CryptoErr("Signature is not verified")
      }
      "public key is malicious (substituted)" in {
        val result = sealOffer(contract)
          .copy(offerSeal = changeToMaliciousPubKey(contract.offerSeal))
          .checkOfferSeal[Option]

        result.failed shouldBe CryptoErr("Signature is not verified")
      }
    }
    "return () when offer seal is correct" in {
      val result = sealOffer(contract)
        .checkOfferSeal[Option]

      result.success shouldBe ()
    }
  }

  "ContractRead.ReadOps.isBlankOffer" should {
    "fail" when {
      "offer seal is corrupted" in {
        val result =
          sealOffer(contract)
            .copy(offerSeal = corruptSignature(contract.offerSeal))
            .isBlankOffer[Option]

        result.failed shouldBe CryptoErr("Signature is not verified")
      }
      "public key is malicious (substituted)" in {
        val result = sealOffer(contract)
          .copy(offerSeal = changeToMaliciousPubKey(contract.offerSeal))
          .isBlankOffer[Option]

        result.failed shouldBe CryptoErr("Signature is not verified")
      }
    }
    "return true when offer seal is correct and no participant found" in {
      val result = sealOffer(contract)
        .isBlankOffer[Option]

      result.success shouldBe true
    }
    "return false when offer seal is correct and some participants were found" in {
      val result = sealOffer(signWithParticipants(contract))
        .isBlankOffer[Option]

      result.success shouldBe false
    }
  }

  "ContractRead.ReadOps.participantSigned" should {
    "fail" when {
      "sign is corrupted" in {
        val signedContract = signWithParticipants(contract)
        val corruptedSignedParticipants: Map[Key, Signature] =
          signedContract.participants.map { case (k, s) ⇒ k → corruptSignature(s) }
        val contractWithCorruptedSign = signedContract.copy(participants = corruptedSignedParticipants)

        val result = contractWithCorruptedSign.participantSigned[Option](signedContract.participants.head._1)
        result.failed shouldBe CryptoErr("Signature is not verified")
      }
      "public key is malicious (substituted)" in {
        val signedContract = signWithParticipants(contract)
        val signedParticipantsWithBadPubKey: Map[Key, Signature] =
          signedContract.participants.map { case (k, s) ⇒ k → changeToMaliciousPubKey(s) }
        val contractWithBadPubKeySign = signedContract.copy(participants = signedParticipantsWithBadPubKey)

        val result = contractWithBadPubKeySign.participantSigned[Option](contractWithBadPubKeySign.participants.head._1)
        result.failed shouldBe CryptoErr("Signature is not verified")
      }
      "id of contract is invalid" in {
        val signedContract = signWithParticipants(contract).copy(id = Key.fromString("123123123").get)
        val result = signedContract.participantSigned[Option](signedContract.participants.head._1)
        result.failed shouldBe a[CryptoErr]
      }
    }
    "return false when participant wasn't sign contract" in {
      val signedContract = signWithParticipants(contract)

      val result = contract.participantSigned[Option](signedContract.participants.head._1)
      result.success shouldBe false
    }
    "return true when participant was sign contract success" in {
      val signedContract = signWithParticipants(contract)

      val result = signedContract.participantSigned[Option](signedContract.participants.head._1)
      result.success shouldBe true
    }
  }

  "ContractRead.ReadOps.checkParticipantsSeal" should {
    "fail" when {
      "participant seal is corrupted" in {
        val result =
          sealParticipants(contract)
            .copy(participantsSeal = Some(corruptSignature(contract.offerSeal)))
            .checkParticipantsSeal[Option]

        result.failed shouldBe CryptoErr("Signature is not verified")
      }
      "public key is malicious (substituted)" in {
        val result = sealParticipants(contract)
          .copy(participantsSeal = Some(changeToMaliciousPubKey(contract.offerSeal)))
          .checkParticipantsSeal[Option]

        result.failed shouldBe CryptoErr("Signature is not verified")
      }
    }
    "return None when offer seal is correct" in {
      val result = contract
        .checkParticipantsSeal[Option]

      result.success shouldBe None
    }
    "return Some(()) when offer seal is correct" in {
      val result = sealParticipants(contract)
        .checkParticipantsSeal[Option]

      result.success shouldBe Some(())
    }
  }

  "ContractRead.ReadOps.checkAllParticipants" should {
    "fail" when {
      "number of participants is not enough" in {
        val result = contract.checkAllParticipants[Option]()
        result.failed shouldBe CryptoErr("Wrong number of participants")
      }
      "one signatures are invalid" in {
        val signedContract: BasicContract = signWithParticipants(contract)
        val corruptedSignedParticipants: Map[Key, Signature] =
          signedContract.participants.map {
            case (k, s) ⇒
              k → s.copy(sign = contractOwnerSigner.sign[Option](ByteVector("corruption".getBytes)).success.sign)
          }
        val contractWithCorruptedSign = signedContract.copy(participants = corruptedSignedParticipants)

        val result = contractWithCorruptedSign.checkAllParticipants[Option]()
        result.failed shouldBe CryptoErr("Signature is not verified")
      }
    }
    "success when signatures of all required participants is valid" in {
      val signedContract = signWithParticipants(contract)
      val result = signedContract.checkAllParticipants[Option]()
      result.success shouldBe ()
    }
  }

  "ContractRead.ReadOps.checkExecStateSeal" should {
    "fail" when {
      "execution state seal is corrupted" in {
        val result =
          sealExecState(contract)
            .copy(executionSeal = corruptSignature(contract.executionSeal))
            .checkExecStateSeal[Option]

        result.failed shouldBe CryptoErr("Signature is not verified")
      }
      "public key is malicious (substituted)" in {
        val result = sealExecState(contract)
          .copy(executionSeal = changeToMaliciousPubKey(contract.executionSeal))
          .checkExecStateSeal[Option]

        result.failed shouldBe CryptoErr("Signature is not verified")
      }
    }
    "return () when offer seal is correct" in {
      val result = sealExecState(contract)
        .checkExecStateSeal[Option]

      result.success shouldBe ()
    }
  }

  "ContractRead.ReadOps.isActiveContract" should {
    "fail" when {
      "offer seal is invalid" in {
        val result = sealAll(signWithParticipants(contract))
          .copy(offerSeal = corruptedSignature)
          .isActiveContract[Option]
          .failed

        result shouldBe a[CryptoErr]
      }
      "participants seal is invalid" in {
        val result =
          sealAll(signWithParticipants(contract))
            .copy(participantsSeal = Some(corruptedSignature))
            .isActiveContract[Option]
            .failed

        result shouldBe a[CryptoErr]
      }
      "execution seal is invalid" in {
        val result =
          sealAll(contract)
            .copy(executionSeal = corruptedSignature)
            .isActiveContract[Option]
            .failed

        result shouldBe a[CryptoErr]
      }
      "all seals are correct but participants is not enough" in {
        val contractWith2Participants: BasicContract = signWithParticipants(contract)
        val result =
          sealAll(contractWith2Participants.copy(participants = Map(contractWith2Participants.participants.head)))
            .isActiveContract[Option]

        result.failed shouldBe a[CryptoErr]
      }
    }
    "return true" when {
      "all seals are correct and enough participants" in {
        sealAll(signWithParticipants(contract)).isActiveContract[Option].success shouldBe true
      }
    }

    "return false when all seals are correct, but participant seal is None" in {
      val result =
        sealAll(signWithParticipants(contract))
          .copy(participantsSeal = None)
          .isActiveContract[Option]
          .success

      result shouldBe false
    }
  }

  "ContractRead.ReadOps.checkAllOwnerSeals" should {
    "fail" when {
      "offer seal is invalid" in {
        val result = sealAll(contract)
          .copy(offerSeal = corruptedSignature)
          .checkAllOwnerSeals[Option]
          .failed

        result shouldBe a[CryptoErr]
      }
      "participants seal is invalid" in {
        val result =
          sealAll(contract)
            .copy(participantsSeal = Some(corruptedSignature))
            .checkAllOwnerSeals[Option]
            .failed

        result shouldBe a[CryptoErr]
      }
      "execution seal is invalid" in {
        val result =
          sealAll(contract)
            .copy(executionSeal = corruptedSignature)
            .checkAllOwnerSeals[Option]
            .failed

        result shouldBe a[CryptoErr]
      }
    }
    "return ()" when {
      "all seals are correct" in {
        sealAll(contract).checkAllOwnerSeals[Option].success shouldBe ()
      }
      "all seals are correct, but participant seal is None" in {
        val result =
          sealAll(contract)
            .copy(participantsSeal = None)
            .checkAllOwnerSeals[Option]
            .success

        result shouldBe ()
      }
    }
  }

  "ContractRead.ReadOps.checkPubKey" should {
    "fail when id of contract is invalid" in {
      val signedContract = signWithParticipants(contract).copy(id = Key.fromString("123123123").get)
      val result = signedContract.checkPubKey[Option]
      result.failed shouldBe a[CryptoErr]
    }
    "success" in {
      val signedContract = signWithParticipants(contract)
      val result = signedContract.checkPubKey[Option]
      result.success shouldBe ()
    }
  }

  private def sealOffer(contract: BasicContract)(implicit checker: SignatureChecker): BasicContract =
    WriteOps[Option, BasicContract](contract).sealOffer(contractOwnerSigner).success

  private def sealParticipants(contract: BasicContract)(implicit checker: SignatureChecker): BasicContract =
    WriteOps[Option, BasicContract](contract).sealParticipants(contractOwnerSigner).success

  private def sealExecState(contract: BasicContract)(implicit checker: SignatureChecker): BasicContract =
    WriteOps[Option, BasicContract](contract).sealExecState(contractOwnerSigner).success

  private def sealAll(contract: BasicContract)(implicit checker: SignatureChecker): BasicContract =
    Now(contract)
      .map(sealOffer)
      .map(sealParticipants)
      .map(sealExecState)
      .value

  /**
   * Create ''N'' participants and sigh contract with every created participant.
   */
  private def signWithParticipants(
    contract: BasicContract,
    participantsNumber: Int = 2
  )(implicit checker: SignatureChecker): BasicContract = {
    val signed =
      for {
        index ← 1 to participantsNumber
      } yield {
        val pKeyPair = signAlgo.generateKeyPair[Option](None).success
        val pSigner = signAlgo.signer(pKeyPair)
        val pKadKey = Key.fromKeyPair[Try](pKeyPair).get

        WriteOps[Option, BasicContract](contract).signOffer(pKadKey, pSigner).success
      }
    WriteOps[Option, BasicContract](contract).addParticipants(signed).success
  }

}
