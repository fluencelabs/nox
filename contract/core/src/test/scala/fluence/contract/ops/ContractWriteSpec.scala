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

import cats.instances.option._
import cats.instances.try_._
import fluence.contract.{BasicContract, _}
import fluence.crypto.CryptoError
import fluence.crypto.algorithm.Ecdsa
import fluence.kad.protocol.Key
import org.scalatest.{Matchers, WordSpec}

import scala.util.Try

class ContractWriteSpec extends WordSpec with Matchers {

  import fluence.contract.ops.ContractRead.ReadOps
  import fluence.contract.ops.ContractWrite.WriteOps

  // contract constants
  private val signAlgo = Ecdsa.signAlgo

  import signAlgo.checker
  private val keyPair = signAlgo.generateKeyPair.unsafe(None)
  private val signer = signAlgo.signer(keyPair)
  private val contractKadKey = Key.fromKeyPair.unsafe(keyPair)

  private val contract: BasicContract = BasicContract.offer[Try](contractKadKey, 2, signer).get

  private val participantKeyPair = signAlgo.generateKeyPair.unsafe(None)
  private val participantKey = Key.fromPublicKey.unsafe(participantKeyPair.publicKey)
  private val participantSigner = signAlgo.signer(participantKeyPair)
  private val participantChecker = signAlgo.checker(participantKeyPair.publicKey)

  "sealOffer" should {
    "fail when signing is failed" in {
      val result = WriteOps[Option, BasicContract](contract).sealOffer(signerWithException).failed
      result shouldBe a[CryptoError]
    }

    "sign offer and set signature in contract" in {
      val result = WriteOps[Option, BasicContract](contract).sealOffer(signer).success
      checker(keyPair.publicKey).check[Option](result.offerSeal, result.getOfferBytes).success shouldBe ()
    }
  }

  "signOffer" should {
    "fail when signing is failed" in {

      val result = WriteOps[Option, BasicContract](contract).signOffer(participantKey, signerWithException).failed
      result shouldBe a[CryptoError]
    }

    "sign offer and set participant key and signature to participants" in {
      val result = WriteOps[Option, BasicContract](contract).signOffer(participantKey, participantSigner).success
      result.participants should not be empty
      result.participants.head._1 shouldBe participantKey
      participantChecker
        .check[Option](result.participants.head._2.signature, result.getOfferBytes)
        .success shouldBe ()
    }
  }

  "sealParticipants" should {
    "fail when signing is failed" in {
      val result = WriteOps[Option, BasicContract](contract).sealParticipants(signerWithException).failed
      result shouldBe a[CryptoError]
    }

    "sign participants and set signature in contract" in {
      val contractWithOneParticipant =
        WriteOps[Option, BasicContract](contract).signOffer(participantKey, participantSigner).success
      val result = WriteOps[Option, BasicContract](contractWithOneParticipant).sealParticipants(signer).success

      result.participantsSeal shouldBe defined
      checker(keyPair.publicKey)
        .check[Option](result.participantsSeal.get, result.getParticipantsBytes)
        .success shouldBe ()
    }
  }

  "addParticipants" should {

    "fail" when {
      "not enough participant in contract (zero participants, required 2)" in {
        val result =
          WriteOps[Option, BasicContract](contract).addParticipants(Seq.empty[BasicContract]).failed
        result shouldBe CryptoError("Wrong number of participants")
      }

      "not enough participant in contract (1 participant, required 2)" in {
        val contractWithOneParticipant =
          WriteOps[Option, BasicContract](contract).signOffer(participantKey, participantSigner).success
        val result =
          WriteOps[Option, BasicContract](contractWithOneParticipant)
            .addParticipants(Seq(contractWithOneParticipant))
            .failed

        result shouldBe CryptoError("Wrong number of participants")
      }

    }

    "add signed by participant contract as participants to base contract " in {

      val participantKeyPair2 = signAlgo.generateKeyPair.unsafe(None)
      val participantKey2 = Key.fromPublicKey.unsafe(participantKeyPair2.publicKey)
      val participantSigner2 = signAlgo.signer(participantKeyPair2)
      val participantChecker2 = signAlgo.checker(participantKeyPair2.publicKey)

      val contractWithParticipant1 =
        WriteOps[Option, BasicContract](contract).signOffer(participantKey, participantSigner).success

      val contractWithParticipant2 =
        WriteOps[Option, BasicContract](contract).signOffer(participantKey2, participantSigner2).success

      val result =
        WriteOps[Option, BasicContract](contract)
          .addParticipants(Seq(contractWithParticipant1, contractWithParticipant2))
          .success

      result.participants.size shouldBe 2
      result.participants.keys should contain allOf (participantKey, participantKey2)
      participantChecker
        .check[Option](result.participants.head._2.signature, result.getOfferBytes)
        .success shouldBe ()

      participantChecker2
        .check[Option](result.participants.last._2.signature, result.getOfferBytes)
        .success shouldBe ()

    }
  }

  "sealExecState" should {
    "fail when signing is failed" in {
      val result = WriteOps[Option, BasicContract](contract).sealExecState(signerWithException).failed
      result shouldBe a[CryptoError]
    }

    "sign execution state and set signature in contract" in {
      val result = WriteOps[Option, BasicContract](contract).sealExecState(signer).success
      checker(keyPair.publicKey).check[Option](result.executionSeal, result.getExecutionStateBytes).success shouldBe ()
    }
  }

}
