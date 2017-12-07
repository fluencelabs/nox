package fluence.network

import java.net.InetAddress

import com.google.protobuf.ByteString
import fluence.kad.Key
import fluence.network.client.{ KademliaClient, NetworkClient }
import fluence.network.proto.kademlia.{ Header, KademliaGrpc }
import fluence.network.server.{ KademliaServerImpl, KademliaService, NetworkServer, NodeSecurity }
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{ Milliseconds, Seconds, Span }
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }

import scala.concurrent.duration._

class NetworkSimulationSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {

  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(10, Seconds), Span(250, Milliseconds))

  class Node(val key: Key, val localPort: Int) {

    private val serverBuilder = NetworkServer.builder(localPort)

    private val bsk = ByteString.copyFrom(key.id)
    private val bsip = ByteString.copyFrom(InetAddress.getLocalHost.getAddress)

    private val header = serverBuilder.contact.map(c ⇒ Header(
      from = Some(fluence.network.proto.kademlia.Node(
        id = bsk,
        ip = bsip,
        port = c.port
      )),
      advertize = true
    ))

    private val client = NetworkClient.builder
      .add(KademliaClient.register(header))
      .build

    private val kademliaClient = KademliaClient(client)

    val kad = new KademliaService(
      key,
      serverBuilder.contact,
      kademliaClient,
      KademliaConf(8, 8, 3, 1.second),
      NodeSecurity.canBeSaved[Task](acceptLocal = true))

    val server = serverBuilder
      .add(KademliaGrpc.bindService(new KademliaServerImpl(kad), global))
      .build

    def start() = server.start()

    def shutdown() = server.shutdown(1.second)
  }

  private val servers = (0 to 20).map { n ⇒
    val port = 3200 + n
    new Node(Key.sha1(Integer.toBinaryString(port).getBytes), port)
  }

  "Network simulation" should {
    "launch 20 nodes and join network" in {
      servers.foreach { s ⇒
        s.start().runAsync.futureValue
      }

      val firstContact = servers.head.server.contact.runAsync.futureValue
      val secondContact = servers.tail.head.server.contact.runAsync.futureValue

      servers.foreach{ s ⇒
        println(Console.BLUE + s"Join: ${s.kad.nodeId}" + Console.RESET)
        s.kad.join(Seq(firstContact, secondContact), 8).runAsync.futureValue
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
