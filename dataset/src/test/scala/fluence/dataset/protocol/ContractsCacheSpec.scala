package fluence.dataset.protocol

import fluence.dataset.peer.{ContractRecord, ContractsCache}
import fluence.kad.Key
import fluence.node.storage.{KVStore, TrieMapKVStore}
import monix.eval.Coeval
import org.scalatest.{Matchers, WordSpec}

import scala.concurrent.duration._

class ContractsCacheSpec extends WordSpec with Matchers {

  val nodeId: Key = Key.fromString("node id")

  val store: KVStore[Coeval, Key, ContractRecord[DumbContract]] =
    TrieMapKVStore()

  val cache: ContractsCache[Coeval, DumbContract] =
    new ContractsCache[Coeval, DumbContract](store, DumbContract.ops(nodeId), 1.minute)

  "contracts cache" should {
    "reject caching empty and unsigned contracts" in {

      cache.cache(DumbContract(Key.fromString("reject"), 1)).value shouldBe false
      cache.cache(DumbContract(Key.fromString("reject"), 1, offerSealed = true)).value shouldBe false
      cache.cache(DumbContract(Key.fromString("reject2"), 1, participants = Set(Key.fromString("adf")))).value shouldBe false
      cache.cache(DumbContract(Key.fromString("reject2"), 1, offerSealed = true, participants = Set(Key.fromString("adf")))).value shouldBe false

    }

    "reject caching contracts where node participates" ignore {
      cache.cache(DumbContract(Key.fromString("reject3"), 1, participants = Set(nodeId), participantsSealed = true)).value shouldBe false

    }

    "cache and update correct contracts" ignore {
      val v1 = DumbContract(Key.fromString("accept"), 1, Set(Key.fromString("some node")), participantsSealed = true)
      cache.cache(v1).value shouldBe true

      cache.find(v1.id).value shouldBe Some(v1)

      cache.cache(v1.copy(participants = Set(Key.fromString("another node")))).value shouldBe false

      val v2 = v1.copy(participants = Set(Key.fromString("another node")), version = 2)

      cache.cache(v2).value shouldBe true

      cache.find(v1.id).value shouldBe Some(v2)

    }
  }
}
