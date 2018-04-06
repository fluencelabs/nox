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

import cats.{Id, Monad}
import cats.data.EitherT
import cats.instances.option._
import cats.instances.try_._
import fluence.contract.BasicContract.{BasicContractRead, BasicContractWrite, ExecutionState, Offer}
import fluence.crypto.algorithm.{CryptoErr, Ecdsa}
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{PubKeyAndSignature, Signature, Signer}
import fluence.kad.protocol.Key
import org.scalatest.{Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.Try

class BasicContractSpec extends WordSpec with Matchers {

  private val signAlgo = Ecdsa.signAlgo
  private val contractOwnerKeyPair = signAlgo.generateKeyPair[Option]().success
  private val signer = signAlgo.signer(contractOwnerKeyPair)
  private val checker = signAlgo.checker(contractOwnerKeyPair.publicKey)
  private val contractKadKey = Key.fromKeyPair.unsafe(contractOwnerKeyPair)

  private val signerWithException = new Signer {
    override def sign[F[_]: Monad](plain: ByteVector): EitherT[F, CryptoErr, Signature] =
      EitherT.leftT(CryptoErr("FAIL"))
    override def publicKey: KeyPair.Public = ???
    override def toString: String = ???
  }

  val contract: BasicContract = BasicContract.offer[Try](contractKadKey, 4, signer).get

  "offer" should {
    "fail" when {
      "'offer' or 'execution state' sealing was failed" in {
        val result = BasicContract.offer[Try](contractKadKey, 4, signerWithException)
        result.failed.get shouldBe a[CryptoErr]
      }
    }
    "return valid contract" in {
      contract.id shouldBe contractKadKey
      contract.publicKey shouldBe signer.publicKey
      contract.offer shouldBe Offer(4)
      checker.check[Option](contract.offerSeal, contract.offer.getBytes).success shouldBe ()
      contract.participants shouldBe empty
      contract.participantsSeal shouldBe None
      contract.executionState shouldBe ExecutionState(0, ByteVector.empty)
      checker.check[Option](contract.executionSeal, contract.executionState.getBytes).success shouldBe ()
    }
  }

  "BasicContractWrite" should {
    "correct perform all methods" in {
      val signature = Signature(ByteVector("some".getBytes))
      BasicContractWrite.setOfferSeal(contract, signature) shouldBe contract.copy(offerSeal = signature)
      BasicContractWrite.setParticipantsSeal(contract, signature) shouldBe contract.copy(
        participantsSeal = Some(signature)
      )
      BasicContractWrite.setExecStateSeal(contract, signature) shouldBe contract.copy(executionSeal = signature)

      val participantKeyPair = signAlgo.generateKeyPair[Option]().success
      val participantKey = Key.fromKeyPair.unsafe(participantKeyPair)
      val participantSignature = PubKeyAndSignature(participantKeyPair.publicKey, signature)
      BasicContractWrite
        .setOfferSignature(contract, participantKey, participantSignature) shouldBe
        contract.copy(participants = Map(participantKey → participantSignature))
    }
  }

  "BasicContractRead" should {
    "correct perform all methods" in {
      BasicContractRead.id(contract) shouldBe contract.id
      BasicContractRead.publicKey(contract) shouldBe contract.publicKey
      BasicContractRead.version(contract) shouldBe contract.executionState.version
      BasicContractRead.participants(contract) shouldBe contract.participants.keySet
      BasicContractRead.participantsRequired(contract) shouldBe contract.offer.participantsRequired
      BasicContractRead.participantSignature(contract, contractKadKey) shouldBe None
      BasicContractRead.getOfferBytes(contract) shouldBe contract.offer.getBytes
      BasicContractRead.offerSeal(contract) shouldBe contract.offerSeal
      BasicContractRead.getParticipantsBytes(contract) shouldBe contract.id.value
      BasicContractRead.participantsSeal(contract) shouldBe contract.participantsSeal
      BasicContractRead.getExecutionStateBytes(contract) shouldBe contract.executionState.getBytes
      BasicContractRead.executionStateSeal(contract) shouldBe contract.executionSeal
    }
  }

}
