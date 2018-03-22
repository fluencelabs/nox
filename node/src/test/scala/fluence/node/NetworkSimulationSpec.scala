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

import java.net.InetAddress

import cats._
import cats.effect.IO
import cats.instances.try_._
import com.typesafe.config.ConfigFactory
import fluence.crypto.SignAlgo
import fluence.crypto.keypair.KeyPair
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.grpc.{KademliaGrpc, KademliaNodeCodec}
import fluence.kad.protocol.{Contact, KademliaRpc, Key}
import fluence.kad.{KademliaConf, KademliaMVar}
import fluence.transport.TransportSecurity
import fluence.transport.grpc.GrpcConf
import fluence.transport.grpc.client.GrpcClient
import fluence.transport.grpc.server.GrpcServer
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{Milliseconds, Seconds, Span}
import org.scalatest.{BeforeAndAfterAll, Matchers, WordSpec}
import slogging._

import scala.concurrent.Future
import scala.concurrent.duration._
import scala.util.Try

class NetworkSimulationSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll with LazyLogging {

  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(10, Seconds), Span(250, Milliseconds))

  implicit val runFuture = new (Future ~> Task) {
    override def apply[A](fa: Future[A]): Task[A] = Task.deferFuture(fa)
  }

  implicit val runTask = new (Task ~> Future) {
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync
  }

  private val algo = SignAlgo.dumb

  import algo.checker

  import KademliaNodeCodec.{codec ⇒ kadCodec}

  private val config = ConfigFactory.load()

  private val clientConf = GrpcConf.read[Try](config).get

  private val serverConf = clientConf.server.get

  class Node(val key: Key, val localPort: Int, kp: KeyPair) {

    private val serverBuilder = GrpcServer.builder(serverConf.copy(port = localPort))

    val contact =
      Contact.buildOwn[Id](InetAddress.getLocalHost.getHostName, localPort, 0, "0", algo.signer(kp)).value.right.get

    private val client = GrpcClient
      .builder(key, IO.pure(contact.b64seed), clientConf)
      .add(KademliaClient.register())
      .build

    private val kademliaClientRpc: Contact ⇒ KademliaRpc[Contact] = c ⇒ {
      logger.trace(s"Contact to get KC for: $c")
      client.service[KademliaRpc[Contact]](c)
    }

    val kad = KademliaMVar(
      key,
      IO.pure(contact),
      kademliaClientRpc,
      KademliaConf(6, 6, 3, 1.second),
      TransportSecurity.canBeSaved[IO](key, acceptLocal = true)
    )

    val server = serverBuilder
      .add(KademliaGrpc.bindService(new KademliaServer(kad.handleRPC), global))
      .onNodeActivity(kad.update(_).toIO, clientConf)
      .build

    def nodeId = key

    def join(peers: Seq[Contact], numOfNodes: Int) = kad.join(peers, numOfNodes)

    def start(): IO[Unit] = server.start

    def shutdown(): IO[Unit] = server.shutdown
  }

  private val servers = (0 to 10).map { n ⇒
    val port = 3000 + n
    val kp = algo.generateKeyPair[Try]().value.get.right.get
    val k = Key.fromKeyPair[Try](kp).get
    new Node(k, port, kp)
  }

  "Network simulation" should {
    "launch 20 nodes and join network" in {
      servers.foreach { s ⇒
        s.start().unsafeRunSync
      }

      val firstContact = servers.head.contact
      val secondContact = servers.last.contact

      servers.foreach { s ⇒
        logger.debug(Console.BLUE + s"Join: ${s.nodeId}" + Console.RESET)
        s.join(Seq(firstContact, secondContact), 6).runAsync.futureValue
        logger.debug(Console.BLUE + "Joined" + Console.RESET)
      }

    }

    "Find itself by lookup iterative" in {
      servers.foreach { s ⇒
        servers.map(_.key).filterNot(_ === s.key).foreach { k ⇒
          val li = s.kad.findNode(k, 8).runAsync.futureValue.map(_.key)

          li should be('nonEmpty)

          //println(Console.MAGENTA + li + Console.RESET)

          li.exists(Key.OrderedKeys.eqv(_, k)) shouldBe true
        }
      }
    }
  }

  override protected def beforeAll(): Unit = {
    LoggerConfig.factory = PrintLoggerFactory
    LoggerConfig.level = LogLevel.WARN
    super.beforeAll()
  }

  override protected def afterAll(): Unit = {
    super.afterAll()
    servers.foreach(_.shutdown())
    LoggerConfig.level = LogLevel.ERROR
  }
}
