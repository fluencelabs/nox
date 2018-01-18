package fluence.dataset.node

import fluence.dataset.node.contract.{ ContractAllocator, ContractRecord, ContractsCache }
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsAllocatorApi, ContractsCacheRpc }
import fluence.kad.Kademlia
import fluence.kad.protocol.Key
import fluence.kad.testkit.TestKademlia
import fluence.storage.{ KVStore, TrieMapKVStore }
import monix.eval.Coeval
import org.scalatest.{ Matchers, WordSpec }

import scala.collection.concurrent.TrieMap
import scala.concurrent.duration._
import scala.language.higherKinds
import scala.util.Random

class DatasetContractsSpec extends WordSpec with Matchers {

  val dsCreated = TrieMap.empty[String, Set[Key]].withDefaultValue(Set.empty)

  def unsafeKey(str: String): Key = Key.fromString[Coeval](str).value

  val createDS: String ⇒ DumbContract ⇒ Coeval[Unit] = id ⇒ c ⇒ {
    if (c.allocationPossible)
      Coeval.evalOnce(dsCreated(id)= dsCreated(id) + c.id)
    else {
      Coeval.raiseError(new IllegalArgumentException("Can't allocate this"))
    }

  }

  case class TestNode(
      kademlia: Kademlia[Coeval, String],
      cacheRpc: ContractsCacheRpc[Coeval, DumbContract],
      allocatorRpc: ContractAllocatorRpc[Coeval, DumbContract],
      allocator: ContractsAllocatorApi[Coeval, DumbContract]
  )

  import TestKademlia.CoevalParallel

  lazy val network: Map[String, TestNode] = {
    val random = new Random(123123)
    TestKademlia.coevalSimulation[String](16, 100, _.b64, Key.sha1[Coeval](random.nextLong().toString.getBytes).value, joinPeers = 3)
  }.map {
    case (contact, kad) ⇒

      val store: KVStore[Coeval, Key, ContractRecord[DumbContract]] =
        TrieMapKVStore()

      contact -> TestNode(
        kad,
        new ContractsCache[Coeval, DumbContract](kad.nodeId, store, DumbContract.ops(kad.nodeId), 1.minute),
        new ContractAllocator[Coeval, DumbContract](store, DumbContract.ops(kad.nodeId), createDS(contact)),
        new DatasetContracts[Coeval, DumbContract, String](
          kad.nodeId,
          store,
          DumbContract.ops(kad.nodeId),
          createDS(contact),
          10,
          _ ⇒ 20,
          1.second,
          kad
        ) {

          override def cacheRpc(contact: String): ContractsCacheRpc[Coeval, DumbContract] =
            network(contact).cacheRpc

          override def allocatorRpc(contact: String): ContractAllocatorRpc[Coeval, DumbContract] =
            network(contact).allocatorRpc

        }
      )
  }

  "contract allocator api" should {
    "place a contract on single node" in {
      val contract = DumbContract(unsafeKey("dumb0"), 1, offerSealed = true)

      val allocated = network.head._2.allocator.allocate(contract, dc ⇒ Coeval.eval(dc.copy(participantsSealed = true))).value

      allocated.participants.size shouldBe 1

      network.head._2.allocator.find(contract.id).value shouldBe allocated
    }

    "place a contract on 5 nodes" in {
      val contract = DumbContract(unsafeKey("dumb1"), requiredStorageSize = 5, offerSealed = true, participantsRequired = 5)

      val allocated = network.head._2.allocator.allocate(contract, dc ⇒ Coeval.eval(dc.copy(participantsSealed = true))).value

      allocated.participants.size shouldBe 5

      network.head._2.allocator.find(contract.id).value shouldBe allocated
    }

    "reject unsealed contracts" in {
      val contract = DumbContract(unsafeKey("dumb2"), 5)

      val allocated = network.head._2.allocator.allocate(contract, dc ⇒ Coeval.eval(dc.copy(participantsSealed = false))).attempt.value

      allocated should be.leftSide
    }
  }
}
