package fluence.node

import cats.~>
import fluence.kad.grpc.KademliaGrpc
import fluence.kad.grpc.client.KademliaClient
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{ Contact, Key }
import fluence.transport.TransportSecurity
import fluence.transport.grpc.client.GrpcClient
import fluence.transport.grpc.server.GrpcServer
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import org.scalatest.concurrent.ScalaFutures
import org.scalatest.time.{ Milliseconds, Seconds, Span }
import org.scalatest.{ BeforeAndAfterAll, Matchers, WordSpec }

import scala.concurrent.Future
import scala.concurrent.duration._

class NetworkSimulationSpec extends WordSpec with Matchers with ScalaFutures with BeforeAndAfterAll {

  override implicit def patienceConfig: PatienceConfig = PatienceConfig(Span(10, Seconds), Span(250, Milliseconds))

  implicit val runFuture = new (Future ~> Task) {
    override def apply[A](fa: Future[A]): Task[A] = Task.deferFuture(fa)
  }

  implicit val runTask = new (Task ~> Future) {
    override def apply[A](fa: Task[A]): Future[A] = fa.runAsync
  }

  class Node(val key: Key, val localPort: Int) {

    private val serverBuilder = GrpcServer.builder(localPort)

    private val client = GrpcClient.builder(key, serverBuilder.contact)
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
      .onNodeActivity(kad.update)
      .build

    def nodeId = key

    def join(peers: Seq[Contact], numOfNodes: Int) = kad.join(peers, numOfNodes)

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
