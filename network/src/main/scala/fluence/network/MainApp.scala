package fluence.network

import java.net.InetAddress
import java.nio.ByteBuffer
import java.security.MessageDigest

import fluence.kad._
import fluence.network.proto.kademlia.KademliaGrpc
import io.grpc.ServerBuilder
import monix.execution.Scheduler.Implicits.global
import cats.syntax.show._
import org.slf4j.LoggerFactory

import scala.io.StdIn
import scala.util.{ Failure, Success }

object MainApp extends App {

  val log = LoggerFactory.getLogger(getClass)

  println("port?")

  val port = StdIn.readInt()

  val addr = InetAddress.getLocalHost

  val md = MessageDigest.getInstance("SHA-1")
  val key = Key(md.digest(Array.concat(ByteBuffer.allocate(Integer.BYTES).putInt(port).array(), addr.getAddress)))

  log.info("Key: " + key)
  log.info("Key size: " + key.id.length)

  val kad = new KademliaG(key, Contact(addr, port))

  val server = ServerBuilder
    .forPort(port)
    .addService(
      KademliaGrpc.bindService(new KademliaService(kad), global)
    )
    .build.start

  sys.addShutdownHook {
    log.warn("*** shutting down gRPC server since JVM is shutting down")
    server.shutdown()
    log.warn("*** server shut down")
  }

  while (true) {

    println("join(j) / lookup(l)")

    StdIn.readLine() match {
      case "j" | "join" ⇒
        println("join port?")
        val p = StdIn.readInt()
        kad.join(Seq(Contact(addr, p))).runAsync.onComplete{
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

  server.awaitTermination()
}
