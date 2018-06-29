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

package fluence.kad

import java.net.InetAddress
import java.security.SecureRandom

import cats._
import cats.effect.IO
import cats.instances.try_._
import cats.syntax.eq._
import com.typesafe.config.ConfigFactory
import fluence.crypto.KeyPair
import fluence.kad.grpc.client.KademliaClientGrpc
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.grpc.KademliaGrpcUpdate
import fluence.kad.protocol.{Contact, ContactSecurity, KademliaRpc, Key}
import fluence.kad.protobuf.grpc.KademliaGrpc
import fluence.transport.grpc.GrpcConf
import fluence.transport.grpc.client.GrpcClient
import fluence.transport.grpc.server.GrpcServer
import monix.execution.Scheduler.Implicits.global
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{Milliseconds, Seconds, Span}
import org.scalatest.{BeforeAndAfterAll, Matchers, WordSpec}
import slogging._

import scala.concurrent.duration._
import scala.util.Try

class NetworkSimulationSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll with LazyLogging {

  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(15, Seconds), Span(250, Milliseconds))

  import fluence.crypto.DumbCrypto.signAlgo

  import KademliaNodeCodec.{pureCodec ⇒ kadCodec}
  import signAlgo.checker

  private val config = ConfigFactory.load()

  private val clientConf = GrpcConf.read[Try](config).get

  private val serverConf = clientConf.server.get

  class Node(val key: Key, val localPort: Int, kp: KeyPair) {

    private val serverBuilder = GrpcServer.builder(serverConf.copy(port = localPort))

    val contact =
      Contact
        .buildOwn(
          InetAddress.getLocalHost.getHostName,
          localPort,
          Some(localPort + 123),
          0,
          "0",
          signAlgo.signer(kp)
        )
        .unsafe(())

    private val client = GrpcClient
      .builder(key, IO.pure(contact.b64seed), clientConf)
      .add(KademliaClientGrpc.register())
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
      ContactSecurity.check[IO](key, acceptLocal = true)
    )

    val server = serverBuilder
      .add(KademliaGrpc.bindService(new KademliaServer(kad.handleRPC), global))
      .onCall(
        KademliaGrpcUpdate.grpcCallback(kad.update(_).map(_ ⇒ ()).toIO(global), clientConf)
      )
      .build

    def nodeId = key

    def join(peers: Seq[Contact], numOfNodes: Int) = kad.join(peers, numOfNodes)

    def start(): IO[Unit] = server.start

    def shutdown(): IO[Unit] = server.shutdown
  }

  private val servers = (0 to 20).map { n ⇒
    val port = 3000 + n
    val kp = signAlgo.generateKeyPair.unsafe(Some(new SecureRandom().generateSeed(20)))
    val k = Key.fromKeyPair.unsafe(kp)
    new Node(k, port, kp)
  }.toVector

  "Network simulation" should {
    "launch 20 nodes and join network" in {
      servers.foreach { s ⇒
        s.start().unsafeRunSync
      }

      val firstContact = servers.head.contact
      val secondContact = servers.last.contact

      firstContact shouldNot be(secondContact)

      val seedContacts = Seq(firstContact, secondContact)

      servers.foreach { s ⇒
        logger.debug(Console.BLUE + s"Join: ${s.nodeId}" + Console.RESET)

        seedContacts.exists(_.publicKey != s.contact.publicKey) shouldBe true

        s.join(seedContacts, 6).runAsync.futureValue shouldBe true

        // Check that RoutingTable is not empty after join
        s.kad.lookupIterative(servers.last.key, 5).runAsync.futureValue should be('nonEmpty)
      }

    }

    "find itself by lookup iterative" in {
      //LoggerConfig.level = LogLevel.INFO
      servers.take(10).foreach { s ⇒
        servers.reverseIterator.map(_.key).filter(_ =!= s.key).slice(5, 15).foreach { k ⇒
          logger.debug(s"Trying to find $k with ${s.key}")

          val li = s.kad.findNode(k, 8).runAsync.futureValue.map(_.key)

          li shouldBe defined

          //println(Console.MAGENTA + li + Console.RESET)

          li.exists(Key.OrderedKeys.eqv(_, k)) shouldBe true
          logger.debug(Console.GREEN + "Found" + Console.RESET)
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
