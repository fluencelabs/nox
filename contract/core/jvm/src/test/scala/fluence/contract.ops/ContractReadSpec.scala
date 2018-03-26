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
import cats.instances.option._
import cats.instances.try_._
import fluence.contract.BasicContract
import fluence.crypto.algorithm.{CryptoErr, Ecdsa}
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{Signature, SignatureChecker}
import fluence.kad.protocol.Key
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.util.Try

class ContractReadSpec extends WordSpec with Matchers {

  private val seed = "seed".getBytes()
  private val keyPair = KeyPair.fromBytes(seed, seed)
  private val signAlgo = Ecdsa.signAlgo
  private val signer = signAlgo.signer(keyPair)
  private val kadKey = Key.fromKeyPair[Try](keyPair).get

  import ContractRead._
  import ContractWrite._
  import signAlgo.checker

  val contract: BasicContract = BasicContract.offer(kadKey, 2, signer).get

  // todo finish

  "ContractRead.ReadOps.checkOfferSeal" should {}

  "ContractRead.ReadOps.checkOfferSignature" should {}

  "ContractRead.ReadOps.isBlankOffer" should {}

  "ContractRead.ReadOps.participantSigned" should {
    "fail" when {
      "sign isn't valid" in {
        val signedContract = signWithParticipants(contract)
        val corruptedSignedParticipants: Map[Key, Signature] =
          signedContract.participants.map {
            case (k, s) ⇒ k → s.copy(sign = signer.sign[Option](ByteVector("corruption".getBytes)).success.sign)
          }
        val contractWithCorruptedSign = signedContract.copy(participants = corruptedSignedParticipants)

        val result = contractWithCorruptedSign.participantSigned[Option](signedContract.participants.head._1)
        result.failed shouldBe CryptoErr("Signature is not verified")
      }
      "public key is malicious (substituted)" in {
        val signedContract = signWithParticipants(contract)
        val maliciousPubKey = signAlgo.generateKeyPair[Option]().success.publicKey
        val signedParticipantsWithBadPubKey: Map[Key, Signature] =
          signedContract.participants.map { case (k, s) ⇒ k → s.copy(publicKey = maliciousPubKey) }
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

  "ContractRead.ReadOps.checkParticipantsSeal" should {}

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
            case (k, s) ⇒ k → s.copy(sign = signer.sign[Option](ByteVector("corruption".getBytes)).success.sign)
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

  "ContractRead.ReadOps.checkExecStateSeal" should {}

  "ContractRead.ReadOps.checkExecStateSignature" should {}

  "ContractRead.ReadOps.isActiveContract" should {}

  "ContractRead.ReadOps.checkAllSeals" should {}

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

  implicit class GetEitherTValue[E, V](origin: EitherT[Option, E, V]) {

    def success: V =
      origin.value.get.right.get

    def failed: E =
      origin.value.get.left.get
  }

}
