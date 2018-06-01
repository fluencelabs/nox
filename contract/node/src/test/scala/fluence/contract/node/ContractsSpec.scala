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

import java.time.Clock

import cats.effect.{IO, LiftIO}
import cats.implicits.catsStdShowForString
import cats.instances.try_._
import fluence.contract.BasicContract
import fluence.contract.client.Contracts
import fluence.contract.node.cache.ContractRecord
import fluence.contract.protocol.{ContractAllocatorRpc, ContractsCacheRpc}
import fluence.crypto.KeyPair
import fluence.kad.Kademlia
import fluence.kad.protocol.Key
import fluence.kad.testkit.TestKademlia
import fluence.kvstore.{InMemoryKVStore, ReadWriteKVStore}
import monix.eval.Coeval
import org.scalatest.{Matchers, WordSpec}

import scala.collection.concurrent.TrieMap
import scala.collection.mutable
import scala.concurrent.duration._
import scala.language.higherKinds
import scala.util.Random

class ContractsSpec extends WordSpec with Matchers {

  private val clock = Clock.systemUTC()

  val dsCreated: mutable.Map[String, Set[Key]] =
    TrieMap.empty[String, Set[Key]].withDefaultValue(Set.empty)
  import fluence.crypto.DumbCrypto.signAlgo
  import signAlgo.checker

  def unsafeKey(str: String): Key = Key.fromStringSha1.unsafe(str)

  implicit object CoevalLift extends LiftIO[Coeval] {
    override def liftIO[A](ioa: IO[A]): Coeval[A] = Coeval(ioa.unsafeRunSync())
  }

  val createDS: String ⇒ BasicContract ⇒ IO[Boolean] = id ⇒
    c ⇒
      if (c.executionState.version == 0)
        IO.pure(dsCreated(id) = dsCreated(id) + c.id).map(_ ⇒ true)
      else {
        IO.pure(false)
  }

  val checkAllocationPossible: BasicContract ⇒ Coeval[Unit] = c ⇒
    if (c.executionState.version == 0)
      Coeval.unit
    else {
      Coeval.raiseError(new IllegalArgumentException("Can't allocate this!"))
  }

  case class TestNode(
    kademlia: Kademlia[Coeval, String],
    cacheRpc: ContractsCacheRpc[BasicContract],
    allocatorRpc: ContractAllocatorRpc[BasicContract],
    allocator: Contracts[Coeval, BasicContract]
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
      val store: ReadWriteKVStore[Key, ContractRecord[BasicContract]] =
        InMemoryKVStore[Key, ContractRecord[BasicContract]]

      contact -> TestNode(
        kad,
        new ContractsCache[Coeval, BasicContract](
          kad.nodeId,
          store,
          1.second,
          clock
        ),
        new ContractAllocator[BasicContract](
          kad.nodeId,
          store,
          createDS(contact),
          _ ⇒ IO(true),
          signer,
          clock
        ),
        Contracts[Coeval, Coeval, BasicContract, String](
          10,
          _ ⇒ 20,
          kad,
          network(_).cacheRpc,
          network(_).allocatorRpc
        )
      )
  }

  def offer(seed: String, participantsRequired: Int = 1): BasicContract = {
    val s = offerSigner(seed)
    BasicContract.offer(Key.fromPublicKey.unsafe(s.publicKey), participantsRequired, s).get
  }

  def offerSigner(seed: String) = {
    signAlgo.signer(KeyPair.fromBytes(seed.getBytes(), seed.getBytes()))
  }

  "contract allocator api" should {
    "place a contract on single node" in {
      val contract = offer("dumb0")

      import fluence.contract.ops.ContractWrite._

      val signer = offerSigner("dumb0")

      val allocated =
        network.head._2.allocator
          .allocate(contract, dc ⇒ WriteOps[Coeval, BasicContract](dc).sealParticipants(signer).leftMap(_.message))
          .value
          .map(_.right.get)
          .value

      allocated.id shouldBe contract.id

      allocated.participants.size shouldBe 1

      val participants = allocated.participants.keySet

      network.values.filter(n ⇒ participants(n.kademlia.nodeId)).foreach {
        _.cacheRpc.find(contract.id).unsafeRunSync() shouldBe defined
      }

      network.head._2.allocator.find(contract.id).value.value.right.get shouldBe allocated
    }

    "place a contract on 5 nodes" in {
      val contract = offer("dumb1", participantsRequired = 5)

      import fluence.contract.ops.ContractWrite._

      val signer = offerSigner("dumb1")

      val allocated =
        network.head._2.allocator
          .allocate(contract, dc ⇒ WriteOps[Coeval, BasicContract](dc).sealParticipants(signer).leftMap(_.message))
          .value
          .map(_.right.get)
          .value

      allocated.participants.size shouldBe 5

      network.head._2.allocator.find(contract.id).value.value.right.get shouldBe allocated
    }

    "reject unsealed contracts" in {
      val contract = offer("dumb2", 5)

      import fluence.contract.ops.ContractWrite._

      val signer = offerSigner("dumb2")

      val allocated =
        network.head._2.allocator
          .allocate(contract, dc ⇒ WriteOps[Coeval, BasicContract](dc).sealParticipants(signer).leftMap(_.message))
          .value
          .attempt
          .value

      allocated should be.leftSide
    }
  }
}
