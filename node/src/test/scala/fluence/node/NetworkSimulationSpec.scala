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

import cats.~>
import cats.instances.try_._
import com.typesafe.config.ConfigFactory
import fluence.kad.grpc.{ KademliaGrpc, KademliaNodeCodec }
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{ Contact, Key }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.{ GrpcClient, GrpcClientConf }
import fluence.transport.grpc.server.GrpcServer
import monix.eval.{ Coeval, Task }
import monix.execution.Scheduler.Implicits.global
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{ Milliseconds, Seconds, Span }
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.util.Try

class NetworkSimulationSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {

  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(10, Seconds), Span(250, Milliseconds))

  implicit val runFuture = new (Future ~> Task) {
    override def apply[A](fa: Future[A]): Task[A] = Task.deferFuture(fa)
  }

  implicit val runTask = new (Task ~> Future) {
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync
  }

  implicit val kadCodec = KademliaNodeCodec[Task]

  class Node(val key: Key, val localPort: Int) {

    private val serverBuilder = GrpcServer.builder(localPort)

    private val clientConf = GrpcClientConf.read[Try](ConfigFactory.load()).get

    private val client = GrpcClient.builder(key, serverBuilder.contact, clientConf)
      .add(KademliaClient.register[Task]())
      .build

    val kademliaClient = client.service[KademliaClient[Task]] _

    val kad = new KademliaService(
      key,
      serverBuilder.contact,
      kademliaClient,
      KademliaConf(8, 8, 3, 1.second),

      TransportSecurity.canBeSaved[Task](key, acceptLocal = true))

    val server = serverBuilder
      .add(KademliaGrpc.bindService(new KademliaServer(kad.handleRPC), global))
      .onNodeActivity(kad.update, clientConf)
      .build

    def nodeId = key

    def join(peers: Seq[Contact], numOfNodes: Int) = kad.join(peers, numOfNodes)

    def start() = server.start()

    def shutdown() = server.shutdown(1.second)
  }

  private val servers = (0 to 20).map { n ⇒
    val port = 3200 + n
    new Node(Key.sha1[Coeval](Integer.toBinaryString(port).getBytes).value, port)
  }

  "Network simulation" should {
    "launch 20 nodes and join network" in {
      servers.foreach { s ⇒
        s.start().runAsync.futureValue
      }

      val firstContact = servers.head.server.contact.runAsync.futureValue
      val secondContact = servers.tail.head.server.contact.runAsync.futureValue

      servers.foreach{ s ⇒
        println(Console.BLUE + s"Join: ${s.nodeId}" + Console.RESET)
        s.join(Seq(firstContact, secondContact), 8).runAsync.futureValue
      }

    }

    "Find itself by lookup iterative" in {
      servers.foreach { s ⇒
        servers.map(_.key).filterNot(_ === s.key).foreach { k ⇒
          val li = s.kad.handleRPC.lookupIterative(k, 8).runAsync
            .futureValue.map(_.key)

          li should be('nonEmpty)

          //println(Console.MAGENTA + li + Console.RESET)

          li.exists(Key.OrderedKeys.eqv(_, k)) shouldBe true
        }
      }
    }
  }

  override protected def afterAll(): Unit = {
    super.afterAll()
    servers.foreach(_.shutdown())
  }
}
