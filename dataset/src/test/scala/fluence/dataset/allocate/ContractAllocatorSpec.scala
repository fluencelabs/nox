package fluence.dataset.allocate

import cats.effect.IO
import fluence.kad.Key
import fluence.node.storage.{ KVStore, TrieMapKVStore }
import org.scalatest.{ Matchers, WordSpec }
import scala.language.higherKinds
import scala.concurrent.duration._

class ContractAllocatorSpec extends WordSpec with Matchers {
  @volatile var denyDS: Set[Key] = Set.empty

  @volatile var dsCreated: Set[Key] = Set.empty

  val nodeId: Key = Key.XorDistanceMonoid.empty

  val createDS: DumbContract ⇒ IO[Unit] = c ⇒ {
    if (denyDS(c.id)) IO.raiseError(new IllegalArgumentException(s"Can't create dataset for ${c.id}"))
    else IO(dsCreated += c.id)
  }

  val store: KVStore[IO, Key, ContractRecord[DumbContract]] =
    TrieMapKVStore()

  val allocator: ContractAllocatorRPC[IO, DumbContract] = new ContractAllocator[IO, DumbContract](
    store, DumbContract.ops(nodeId), createDS
  )

  val cache: ContractsCache[IO, DumbContract] = new ContractsCache[IO, DumbContract](store, DumbContract.ops(nodeId), 1.minute)

  "contract allocator" should {

    "reject offer with wrong signature" in {
      val contract = DumbContract(Key.fromString("dumb0"), 1)
      allocator.offer(contract).attempt.unsafeRunSync().isLeft shouldBe true
    }

    "reject offer with unsufficent resources" in {
      val contract = DumbContract(Key.fromString("should reject"), 1, offerSealed = true, allocationPossible = false)
      allocator.offer(contract).attempt.unsafeRunSync().isLeft shouldBe true
    }

    "accept offer (idempotently)" in {
      val contract = DumbContract(Key.fromString("should accept"), 1, offerSealed = true)
      val accepted = allocator.offer(contract).unsafeRunSync()

      accepted.participants should contain(nodeId)

      allocator.offer(contract).unsafeRunSync() shouldBe accepted

      store.get(accepted.id).unsafeRunSync().contract shouldBe accepted.copy(participants = Set.empty)
    }

    "update accepted offer" in {
      val contract = DumbContract(Key.fromString("should update"), 1, offerSealed = true)
      val v1 = allocator.offer(contract).unsafeRunSync()

      v1.participants should contain(nodeId)
      v1.size shouldBe 1

      store.get(v1.id).unsafeRunSync().contract shouldBe v1.copy(participants = Set.empty)

      val v2 = allocator.offer(contract.copy(size = 2)).unsafeRunSync()
      v2.participants should contain(nodeId)
      v2.size shouldBe 2
    }

    "not return (accepted) offer from cache" in {
      val contract = DumbContract(Key.fromString("should accept, but not return"), 1, offerSealed = true)
      val accepted = allocator.offer(contract).unsafeRunSync()

      cache.find(accepted.id).unsafeRunSync() should be('empty)
    }

    "reject allocation when not in the list of participants" in {
      val contract = DumbContract(Key.fromString("should not allocate, as not a participant"), 1, offerSealed = true)
      allocator.allocate(contract).attempt.unsafeRunSync().isLeft shouldBe true

      val c2 = DumbContract(
        Key.fromString("should not allocate, as not a participant, even with a list of participants"),
        1, offerSealed = true,
        participants = Set(contract.id))
      allocator.allocate(c2).attempt.unsafeRunSync().isLeft shouldBe true
    }

    "reject allocation on the same conditions as it was an offer" in {
      val offer = DumbContract(Key.fromString("should accept offer, but reject allocation"), 1, offerSealed = true)
      val accepted = allocator.offer(offer).unsafeRunSync()

      allocator.allocate(accepted).attempt.unsafeRunSync().isLeft shouldBe true
      allocator.allocate(accepted.copy(participantsSealed = true, allocationPossible = false)).attempt.unsafeRunSync().isLeft shouldBe true

      denyDS += offer.id
      allocator.allocate(accepted.copy(participantsSealed = true)).attempt.unsafeRunSync().isLeft shouldBe true
    }

    "allocate (idempotently) and return from cache" in {
      val offer = DumbContract(Key.fromString("should accept offer and allocate"), 1, offerSealed = true)
      val accepted = allocator.offer(offer).unsafeRunSync().copy(participantsSealed = true)
      val contract = allocator.allocate(accepted).unsafeRunSync()

      contract shouldBe accepted

      cache.find(contract.id).unsafeRunSync() shouldBe Some(contract)

      dsCreated should contain(contract.id)
    }
  }
}
