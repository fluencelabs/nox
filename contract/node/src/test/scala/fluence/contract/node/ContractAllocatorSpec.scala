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

package fluence.contract.node

import cats.effect.IO
import cats.instances.try_._
import fluence.contract.BasicContract
import fluence.contract.node.cache.ContractRecord
import fluence.contract.protocol.ContractAllocatorRpc
import fluence.crypto.SignAlgo
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.Signature
import fluence.kad.protocol.Key
import fluence.storage.{ KVStore, TrieMapKVStore }
import org.scalatest.{ Matchers, WordSpec }
import scodec.bits.ByteVector

import scala.concurrent.duration._
import scala.language.higherKinds
import scala.util.Try

class ContractAllocatorSpec extends WordSpec with Matchers {
  @volatile var denyDS: Set[Key] = Set.empty

  @volatile var dsCreated: Set[Key] = Set.empty

  val keypair = KeyPair.fromBytes(Array.emptyByteArray, Array.emptyByteArray)

  val nodeId: Key = Key.fromPublicKey[IO](keypair.publicKey).unsafeRunSync()

  val signAlgo = SignAlgo.dumb
  val signer = signAlgo.signer(keypair)

  import signAlgo.checker

  val createDS: BasicContract ⇒ IO[Unit] = c ⇒ {
    if (denyDS(c.id)) IO.raiseError(new IllegalArgumentException(s"Can't create dataset for ${c.id}"))
    else IO(dsCreated += c.id)
  }

  val checkAllocationPossible: BasicContract ⇒ IO[Unit] =
    c ⇒
      if (c.executionState.version == 0) IO.unit
      else IO.raiseError(new IllegalArgumentException("allocation not possible"))

  val store: KVStore[IO, Key, ContractRecord[BasicContract]] =
    TrieMapKVStore()

  val allocator: ContractAllocatorRpc[IO, BasicContract] = new ContractAllocator[IO, BasicContract](
    nodeId,
    store,
    createDS,
    checkAllocationPossible,
    signer
  )

  val cache: ContractsCache[IO, BasicContract] =
    new ContractsCache[IO, BasicContract](nodeId, store, 1.minute)

  def offer(seed: String, participantsRequired: Int = 1): BasicContract = {
    val s = offerSigner(signAlgo, seed)
    BasicContract.offer[Try](Key.fromPublicKey[IO](s.publicKey).unsafeRunSync(), participantsRequired, s).get
  }

  def offerSigner(signAlgo: SignAlgo, seed: String) = {
    signAlgo.signer(KeyPair.fromBytes(seed.getBytes(), seed.getBytes()))
  }

  "contract allocator" should {

    "reject offer with wrong signature" in {
      val contract = offer("dumb0").copy(offerSeal = Signature(KeyPair.Public(ByteVector.empty), ByteVector.empty))
      allocator.offer(contract).attempt.unsafeRunSync().isLeft shouldBe true
    }

    "reject offer with unsufficent resources" in {
      val contract = offer("should reject").copy(
        executionState = BasicContract.ExecutionState(version = -1, merkleRoot = ByteVector.empty)
      )
      allocator.offer(contract).attempt.unsafeRunSync().isLeft shouldBe true
    }

    "accept offer (idempotently)" in {
      val contract = offer("should accept")
      val accepted = allocator.offer(contract).unsafeRunSync()

      accepted.participants.keySet should contain(nodeId)

      allocator.offer(contract).unsafeRunSync() shouldBe accepted

      store.get(accepted.id).unsafeRunSync().contract shouldBe accepted.copy(participants = Map.empty)
    }

    "update accepted offer" in {
      val contract = offer("should update")
      val v1 = allocator.offer(contract).unsafeRunSync()

      v1.participants.keySet should contain(nodeId)
      /*
      TODO: update the test when there's any data in the contract
      v1.requiredStorageSize shouldBe 1

      store.get(v1.id).unsafeRunSync().contract shouldBe v1.copy(participants = Set.empty)

      val v2 = allocator.offer(contract.copy(requiredStorageSize = 2)).unsafeRunSync()
      v2.participants should contain(nodeId)
      v2.requiredStorageSize shouldBe 2
     */
    }

    "not return (accepted) offer from cache" in {
      val contract = offer("should accept, but not return")
      val accepted = allocator.offer(contract).unsafeRunSync()

      cache.find(accepted.id).unsafeRunSync() should be('empty)
    }

    "reject allocation when not in the list of participants" in {
      val contract = offer("should not allocate, as not a participant")
      allocator.allocate(contract).attempt.unsafeRunSync().isLeft shouldBe true

      val s2 = offerSigner(signAlgo, "signer some")
      import fluence.contract.ops.ContractWrite._

      val c2 = offer("should not allocate, as not a participant, even with a list of participants")
        .signOffer(Key.fromPublicKey[IO](s2.publicKey).unsafeRunSync(), s2)
        .get

      allocator.allocate(c2).attempt.unsafeRunSync().isLeft shouldBe true
    }

    "reject allocation on the same conditions as it was an offer" in {
      val offerC = offer("should accept offer, but reject allocation")
      val signer = offerSigner(signAlgo, "should accept offer, but reject allocation")
      val accepted = allocator.offer(offerC).unsafeRunSync()

      import fluence.contract.ops.ContractWrite._

      allocator.allocate(accepted).attempt.unsafeRunSync().isLeft shouldBe true
      allocator
        .allocate(
          accepted
            .sealParticipants(signer)
            .get
            .copy(executionState = BasicContract.ExecutionState(version = -1, merkleRoot = ByteVector.empty))
        )
        .attempt
        .unsafeRunSync()
        .isLeft shouldBe true

      denyDS += offerC.id
      allocator.allocate(accepted.sealParticipants(signer).get).attempt.unsafeRunSync().isLeft shouldBe true
    }

    "allocate (idempotently) and return from cache" in {
      import fluence.contract.ops.ContractWrite._

      val offerC = offer("should accept offer and allocate")
      val signer = offerSigner(signAlgo, "should accept offer and allocate")
      val accepted = allocator.offer(offerC).unsafeRunSync().sealParticipants(signer).get
      val contract = allocator.allocate(accepted).unsafeRunSync()

      contract shouldBe accepted

      cache.find(contract.id).unsafeRunSync() shouldBe Some(contract)

      dsCreated should contain(contract.id)
    }
  }
}
