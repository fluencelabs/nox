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

import java.nio.ByteBuffer

import fluence.client.ClientComposer
import cats.instances.future._
import cats.~>
import com.typesafe.config.{ ConfigFactory, ConfigValueFactory }
import fluence.crypto.keypair.KeyPair
import fluence.crypto.signature
import fluence.crypto.signature.SignatureChecker
import fluence.dataset.BasicContract
import fluence.dataset.protocol.ContractsApi
import fluence.info.NodeInfo
import fluence.kad.protocol.Key
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

  private val pureClient = ClientComposer.grpc[Future](GrpcClient.builder)

  private val config = ConfigFactory.load()

  private val servers = (0 to 20).map { n ⇒
    val port = 3100 + n

    val seedBytes = {
      val bb = ByteBuffer.allocate(Integer.BYTES)
      bb.putInt(port)
      bb.array()
    }

    new NodeComposer(
      KeyPair.fromBytes(seedBytes, seedBytes),
      config
        .withValue("fluence.transport.grpc.server.localPort", ConfigValueFactory.fromAnyRef(port))
        .withValue("fluence.transport.grpc.server.externalPort", ConfigValueFactory.fromAnyRef(null))
        .withValue("fluence.transport.grpc.server.acceptLocal", ConfigValueFactory.fromAnyRef(true)),
      () ⇒ Task.now(NodeInfo("test")),
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

      val contractsApi = pureClient.service[ContractsApi[Future, BasicContract]](servers.head.server.flatMap(_.contact).runAsync.futureValue)

      contractsApi.find(Key.fromString[Future]("hi there").futureValue).failed.futureValue

      val seed = Array[Byte](1, 2, 3, 4, 5)
      val kp = KeyPair.fromBytes(seed, seed)
      val key = Key.fromKeyPair[Future](kp).futureValue
      val signer = new signature.Signer.DumbSigner(kp)
      val offer = BasicContract.offer(key, participantsRequired = 4, signer = signer).futureValue

      offer.checkOfferSeal(SignatureChecker.DumbChecker).futureValue shouldBe true

      val accepted = contractsApi.allocate(offer, bc ⇒
        {
          Future successful bc.sealParticipants(signer).futureValue
        }
      ).futureValue

      accepted.participants.size shouldBe 4

      contractsApi.find(key).futureValue shouldBe accepted

    }
  }

  override protected def afterAll(): Unit = {
    super.afterAll()
    servers.foreach(_.server.foreach(_.shutdown(1.second)))
  }

}
