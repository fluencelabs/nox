package fluence.network

import java.net.InetAddress

import fluence.kad._
import fluence.network.proto.kademlia.{ Header, KademliaGrpc }
import monix.execution.Scheduler.Implicits.global
import cats.syntax.show._
import fluence.network.client.{ KademliaClient, NetworkClient }
import fluence.network.server.{ KademliaServerImpl, KademliaService, NetworkServer }
import monix.eval.Task
import org.slf4j.LoggerFactory

import scala.io.StdIn
import scala.util.{ Failure, Random, Success }
import scala.concurrent.duration._

object MainApp extends App {

  val log = LoggerFactory.getLogger(getClass)

  val port100 = Random.nextInt(100)

  val serverBuilder = NetworkServer.builder(1000 + port100, 56000 + port100)

  val kb = Array.ofDim[Byte](100)
  Random.nextBytes(kb)

  val key = Key.sha1(kb)

  val header = Task(Header())

  val client = NetworkClient.builder
    .add(KademliaClient.register(header))
    .build

  val kad = new KademliaService(key, serverBuilder.contact, KademliaClient(client), k = 16, parallelism = 3, pingTimeout = 1.second)

  val server = serverBuilder
    .add(KademliaGrpc.bindService(new KademliaServerImpl(kad), global))
    .build

  sys.addShutdownHook {
    log.warn("*** shutting down gRPC server since JVM is shutting down")
    server.shutdown(10.seconds)
    log.warn("*** server shut down")
  }

  while (true) {

    println("join(j) / lookup(l)")

    StdIn.readLine() match {
      case "j" | "join" ⇒
        println("join port?")
        val p = StdIn.readInt()
        kad.join(Seq(Contact(InetAddress.getLocalHost, p)), 16).runAsync.onComplete{
          case Success(_) ⇒ println("ok")
          case Failure(e) ⇒
            println(e)
            e.printStackTrace()
        }

      case "l" | "lookup" ⇒
        println("lookup myself")
        kad.handleRPC.lookup(Key.XorDistanceMonoid.empty, 10).map(_.map(_.show).mkString("\n")).map(println).runAsync.onComplete{
          case Success(_) ⇒ println("ok")
          case Failure(e) ⇒
            println(e)
            e.printStackTrace()
        }
      case _ ⇒
    }
  }
}
