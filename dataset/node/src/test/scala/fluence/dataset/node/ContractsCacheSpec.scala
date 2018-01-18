package fluence.dataset.node

import fluence.dataset.node.contract.{ ContractRecord, ContractsCache }
import fluence.kad.protocol.Key
import fluence.storage.{ KVStore, TrieMapKVStore }
import monix.eval.Coeval
import org.scalatest.{ Matchers, WordSpec }

import scala.concurrent.duration._

class ContractsCacheSpec extends WordSpec with Matchers {

  def unsafeKey(str: String): Key = Key.fromString[Coeval](str).value

  val nodeId: Key = unsafeKey("node id")

  val store: KVStore[Coeval, Key, ContractRecord[DumbContract]] =
    TrieMapKVStore()

  val cache: ContractsCache[Coeval, DumbContract] =
    new ContractsCache[Coeval, DumbContract](nodeId, store, DumbContract.ops(nodeId), 1.minute)

  "contracts cache" should {
    "reject caching empty and unsigned contracts" in {

      cache.cache(DumbContract(unsafeKey("reject"), 1)).value shouldBe false
      cache.cache(DumbContract(unsafeKey("reject"), 1, offerSealed = true)).value shouldBe false
      cache.cache(DumbContract(unsafeKey("reject2"), 1, participants = Set(unsafeKey("adf")))).value shouldBe false
      cache.cache(DumbContract(unsafeKey("reject2"), 1, offerSealed = true, participants = Set(unsafeKey("adf")))).value shouldBe false

    }

    "reject caching contracts where node participates" in {
      cache.cache(DumbContract(unsafeKey("reject3"), 1, participants = Set(nodeId), participantsSealed = true)).value shouldBe false

    }

    "cache and update correct contracts" in {
      val v1 = DumbContract(unsafeKey("accept"), 1, Set(unsafeKey("some node")), participantsSealed = true)
      cache.cache(v1).value shouldBe true

      cache.find(v1.id).value shouldBe Some(v1)

      cache.cache(v1.copy(participants = Set(unsafeKey("another node")))).value shouldBe false

      val v2 = v1.copy(participants = Set(unsafeKey("another node")), version = 2)

      cache.cache(v2).value shouldBe true

      cache.find(v1.id).value shouldBe Some(v2)

    }
  }
}
