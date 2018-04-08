package fluence.node.grpc

import java.time.Instant

import cats.effect.IO
import cats.instances.try_._
import cats.~>
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.grpc.proxy.ProxyGrpc
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.grpc.{KademliaGrpc, PingRequest}
import fluence.kad.protocol.{Contact, KademliaRpc, Key, Node}
import fluence.proxy.grpc.WebsocketMessage
import io.grpc.inprocess.{InProcessChannelBuilder, InProcessServerBuilder}
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}

import scala.concurrent.Future
import scala.util.{Random, Try}

object SomeMain extends App {

  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.INFO

  type C = Contact

  val algo: SignAlgo = Ecdsa.signAlgo
  import algo.checkerFn
  import fluence.kad.grpc.KademliaNodeCodec.{pureCodec ⇒ nodeCodec}

  val kp = algo.generateKeyPair[Try]().value.get.right.get
  val contact = Contact("", 1, kp.publicKey, 1L, "", "")

  def rndString(size: Int = 10): String = Random.nextString(size)
  def rndNode() = Node[C](Key.fromStringSha1.runF[Try](rndString(10)).get, Instant.now(), contact)

  val RPC = {
    new KademliaRpc[C] {
      override def ping(): IO[Node[C]] = {
        println("PING REQUEST")
        IO(Node[C](Key.fromStringSha1.runF[Try]("123123").get, Instant.now(), contact))
      }

      override def lookup(key: Key, numberOfNodes: Int): IO[Seq[Node[C]]] = IO(List.fill(5)(rndNode()))

      override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): IO[Seq[Node[C]]] =
        IO(List.fill(5)(rndNode()))
    }
  }

  val service = KademliaGrpc.bindService(new KademliaServer(RPC), scala.concurrent.ExecutionContext.global)

  val chs = {

    println("COUNT CHS")

    val inProcessServer = InProcessServerBuilder.forName("in-process")
    List(service).foreach(s ⇒ inProcessServer.addService(s))
    inProcessServer.build().start()

    InProcessChannelBuilder.forName("in-process").build()
  }

  implicit def runFuture: Future ~> IO = new (Future ~> IO) {
    override def apply[A](fa: Future[A]): IO[A] = IO.fromFuture(IO(fa))
  }
  val inProcessGrpc = new ProxyGrpc[IO](List(service), chs)

  val ping = PingRequest()
  val method = KademliaGrpc.METHOD_PING

  val splitted = method.getFullMethodName.split("/").toList

  val msg = WebsocketMessage(splitted(0), splitted(1), ping.toByteString)

  val resp = inProcessGrpc
    .handleMessage(msg.service, msg.method, msg.protoMessage.newInput())
    .flatMap(f ⇒ runFuture(f))
    .unsafeRunSync()

  val resp1 = inProcessGrpc
    .handleMessage(msg.service, msg.method, msg.protoMessage.newInput())
    .flatMap(f ⇒ runFuture(f))
    .unsafeRunSync()

  val resp2 = inProcessGrpc
    .handleMessage(msg.service, msg.method, msg.protoMessage.newInput())
    .flatMap(f ⇒ runFuture(f))
    .unsafeRunSync()

  val classResp = fluence.kad.grpc.Node.parseFrom(resp)
  val classResp1 = fluence.kad.grpc.Node.parseFrom(resp1)
  val classResp2 = fluence.kad.grpc.Node.parseFrom(resp2)

  println("CLASS RESP === " + classResp)
  println("CLASS RESP === " + classResp1)
  println("CLASS RESP === " + classResp)

}
