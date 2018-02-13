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

package fluence.node

import fluence.client.ClientComposer
import cats.instances.future._
import cats.~>
import com.typesafe.config.{ ConfigFactory, ConfigValueFactory }
import fluence.crypto.algorithm.Ecdsa
import fluence.crypto.SignAlgo
import fluence.dataset.BasicContract
import fluence.dataset.client.Contracts
import fluence.kad.protocol.{ Contact, Key }
import fluence.transport.grpc.client.GrpcClient
import monix.eval.Task
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{ Milliseconds, Seconds, Span }
import monix.execution.Scheduler.Implicits.global

import scala.concurrent.Future
import scala.language.higherKinds
import scala.concurrent.duration._

class NodeComposerSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {

  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(10, Seconds), Span(250, Milliseconds))

  private implicit def runId[F[_]]: F ~> F = new (F ~> F) {
    override def apply[A](fa: F[A]): F[A] = fa
  }

  private val algo = new SignAlgo(Ecdsa.ecdsa_secp256k1_sha256)

  private implicit val checker = algo.checker

  private val pureClient = ClientComposer.grpc[Future](GrpcClient.builder)

  private val config = ConfigFactory.load()

  private val servers = (0 to 20).map { n ⇒
    val port = 3100 + n

    new NodeComposer(
      algo.generateKeyPair[Future]().value.futureValue.right.get,
      algo,
      config
        .withValue("fluence.transport.grpc.server.localPort", ConfigValueFactory.fromAnyRef(port))
        .withValue("fluence.transport.grpc.server.externalPort", ConfigValueFactory.fromAnyRef(null))
        .withValue("fluence.transport.grpc.server.acceptLocal", ConfigValueFactory.fromAnyRef(true)),
      "node_cache_" + n
    )
  }

  "Node composer simulation" should {
    "launch 20 nodes and join network" in {
      servers.foreach { s ⇒
        s.server.flatMap(_.start()).runAsync.futureValue
      }

      val firstContact = servers.head.server.flatMap(_.contact).runAsync.futureValue
      val secondContact = servers.tail.head.server.flatMap(_.contact).runAsync.futureValue

      servers.foreach { s ⇒
        s.services.flatMap(_.kademlia.join(Seq(firstContact, secondContact), 8)).runAsync.futureValue
      }

      /*servers.foreach { s ⇒
        servers.map(_.key).filterNot(_ === s.key).foreach { k ⇒
          val li = s.kad.handleRPC.lookupIterative(k, 8).runAsync
            .futureValue.map(_.key)

          li should be('nonEmpty)

          //println(Console.MAGENTA + li + Console.RESET)

          li.exists(Key.OrderedKeys.eqv(_, k)) shouldBe true
        }
      }*/
    }

    "reply to client's commands" in {

      import fluence.dataset.contract.ContractWrite._
      import fluence.dataset.contract.ContractRead._

      val contractsApi = new Contracts[Task, BasicContract, Contact](
        maxFindRequests = 10,
        maxAllocateRequests = _ ⇒ 20,
        checker = algo.checker,
        kademlia = servers.head.services.map(_.kademlia).runAsync.futureValue, // TODO: use client-side Kademlia
        // TODO: store servers in a ~contact=>server map, get them by contact
        cacheRpc = c ⇒ servers.find(_.services.map(_.kademlia).flatMap(_.ownContact).runAsync.futureValue.contact.publicKey.value == c.publicKey.value).get.services.map(_.contractsCache).runAsync.futureValue,
        allocatorRpc = c ⇒ servers.find(_.services.map(_.kademlia).flatMap(_.ownContact).runAsync.futureValue.contact.publicKey.value == c.publicKey.value).get.services.map(_.contractAllocator).runAsync.futureValue
      )

      contractsApi.find(Key.fromString[Future]("hi there").futureValue).failed.runAsync.futureValue

      val kp = algo.generateKeyPair[Future]().value.futureValue.right.get
      val key = Key.fromKeyPair[Future](kp).futureValue
      val signer = algo.signer(kp)
      val offer = BasicContract.offer(key, participantsRequired = 4, signer = signer).futureValue

      offer.checkOfferSeal(algo.checker).futureValue shouldBe true

      // TODO: add test with wrong signature or other errors
      val accepted = contractsApi.allocate(offer, bc ⇒
        {
          Task.now(bc.sealParticipants(signer).futureValue)
        }
      ).runAsync.futureValue

      accepted.participants.size shouldBe 4

      contractsApi.find(key).runAsync.futureValue shouldBe accepted

    }
  }

  override protected def afterAll(): Unit = {
    super.afterAll()
    servers.foreach(_.server.foreach(_.shutdown(1.second)))
  }

}
