package fluence.dataset.node

import cats.instances.try_._
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature.{ SignatureChecker, Signer }
import fluence.dataset.BasicContract
import fluence.dataset.node.contract.ContractRecord
import fluence.dataset.protocol.{ ContractAllocatorRpc, ContractsApi, ContractsCacheRpc }
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

class ContractsSpec extends WordSpec with Matchers {

  val dsCreated = TrieMap.empty[String, Set[Key]].withDefaultValue(Set.empty)

  def unsafeKey(str: String): Key = Key.fromString[Coeval](str).value

  val createDS: String ⇒ BasicContract ⇒ Coeval[Unit] = id ⇒ c ⇒
    if (c.version == 0)
      Coeval.evalOnce(dsCreated(id)= dsCreated(id) + c.id)
    else {
      Coeval.raiseError(new IllegalArgumentException("Can't allocate this"))
    }

  val checkAllocationPossible: BasicContract ⇒ Coeval[Unit] = c ⇒
    if (c.version == 0)
      Coeval.unit
    else {
      Coeval.raiseError(new IllegalArgumentException("Can't allocate this!"))
    }

  case class TestNode(
      kademlia: Kademlia[Coeval, String],
      cacheRpc: ContractsCacheRpc[Coeval, BasicContract],
      allocatorRpc: ContractAllocatorRpc[Coeval, BasicContract],
      allocator: ContractsApi[Coeval, BasicContract]
  )

  import TestKademlia.CoevalParallel

  lazy val network: Map[String, TestNode] = {
    val random = new Random(123123)
    TestKademlia.coevalSimulationKP[String](16, 100, _.b64, {
      val seed = random.nextLong().toString.getBytes
      KeyPair.fromBytes(seed, seed)
    }, joinPeers = 3)
  }.map {
    case (contact, (signer, kad)) ⇒

      val store: KVStore[Coeval, Key, ContractRecord[BasicContract]] =
        TrieMapKVStore()

      val contracts = new Contracts[Coeval, BasicContract, String](
        kad.nodeId,
        store,
        _ ⇒ Coeval.unit,
        createDS(contact),
        10,
        _ ⇒ 20,
        SignatureChecker.DumbChecker,
        signer,
        1.second,
        kad
      ) {

        override def cacheRpc(contact: String): ContractsCacheRpc[Coeval, BasicContract] =
          network(contact).cacheRpc

        override def allocatorRpc(contact: String): ContractAllocatorRpc[Coeval, BasicContract] =
          network(contact).allocatorRpc

      }

      contact -> TestNode(
        kad,
        contracts.cache,
        contracts.allocator,
        contracts
      )
  }

  def offer(seed: String, participantsRequired: Int = 1): BasicContract = {
    val s = offerSigner(seed)
    BasicContract.offer(Key.fromPublicKey[Coeval](s.publicKey).value, participantsRequired, s).get
  }

  def offerSigner(seed: String) = {
    new Signer.DumbSigner(KeyPair.fromBytes(seed.getBytes(), seed.getBytes()))
  }

  "contract allocator api" should {
    "place a contract on single node" in {
      val contract = offer("dumb0")

      import fluence.dataset.contract.ContractWrite._

      val signer = offerSigner("dumb0")

      val allocated = network.head._2.allocator.allocate(contract, dc ⇒ Coeval.eval(dc.sealParticipants(signer).get)).value

      allocated.participants.size shouldBe 1

      network.head._2.allocator.find(contract.id).value shouldBe allocated
    }

    "place a contract on 5 nodes" in {
      val contract = offer("dumb1", participantsRequired = 5)

      import fluence.dataset.contract.ContractWrite._

      val signer = offerSigner("dumb1")

      val allocated = network.head._2.allocator.allocate(contract, dc ⇒ Coeval.eval(dc.sealParticipants(signer).get)).value

      allocated.participants.size shouldBe 5

      network.head._2.allocator.find(contract.id).value shouldBe allocated
    }

    "reject unsealed contracts" in {
      val contract = offer("dumb2", 5)

      import fluence.dataset.contract.ContractWrite._

      val signer = offerSigner("dumb2")

      val allocated = network.head._2.allocator.allocate(contract, dc ⇒ Coeval.eval(dc.sealParticipants(signer).get)).attempt.value

      allocated should be.leftSide
    }
  }
}
