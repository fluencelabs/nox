package fluence.node.grpc

import java.time.Instant

import cats.effect.IO
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.kad.grpc.KademliaGrpc.Kademlia
import fluence.kad.grpc.{KademliaGrpc, PingRequest}
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{Contact, KademliaRpc, Key, Node}
import fluence.proxy.grpc.WebsocketMessage
import cats.instances.try_._
import io.grpc.inprocess.{InProcessChannelBuilder, InProcessServerBuilder}

import scala.util.{Random, Try}

object SomeMain extends App {

  type C = Contact

  val algo: SignAlgo = Ecdsa.signAlgo
  import algo.checkerFn

  import fluence.contract.grpc.BasicContractCodec.{codec ⇒ contractCodec}
  import fluence.kad.grpc.KademliaNodeCodec.{codec ⇒ nodeCodec}
  val keyI = Key.bytesCodec[IO]

  import keyI.inverse

  val kp = algo.generateKeyPair[Try]().value.get.right.get
  val contact = Contact("", 1, kp.publicKey, 1L, "", "")

  def rndString(size: Int = 10): String = Random.nextString(size)
  def rndNode() = Node[C](Key.fromString[Try](rndString(10)).get, Instant.now(), contact)

  val RPC = {
    new KademliaRpc[C] {
      override def ping(): IO[Node[C]] = {
        println("PING REQUEST")
        IO(Node[C](Key.fromString[Try]("123123").get, Instant.now(), contact))
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

  val inProcessGrpc = new ProxyGrpc[IO](List(service), chs)

  val ping = PingRequest()
  val method = KademliaGrpc.METHOD_PING

  val splitted = method.getFullMethodName.split("/").toList

  val msg = WebsocketMessage(splitted(0), splitted(1), ping.toByteString)

  val resp = inProcessGrpc.handleMessage(msg.service, msg.method, msg.protoMessage.newInput()).unsafeRunSync()
  val resp1 = inProcessGrpc.handleMessage(msg.service, msg.method, msg.protoMessage.newInput()).unsafeRunSync()
  val resp2 = inProcessGrpc.handleMessage(msg.service, msg.method, msg.protoMessage.newInput()).unsafeRunSync()

  val classResp = fluence.kad.grpc.Node.parseFrom(resp)
  val classResp1 = fluence.kad.grpc.Node.parseFrom(resp1)
  val classResp2 = fluence.kad.grpc.Node.parseFrom(resp2)

  println("CLASS RESP === " + classResp)
  println("CLASS RESP === " + classResp1)
  println("CLASS RESP === " + classResp)

}
