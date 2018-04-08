package fluence.grpc.proxy

import java.io.ByteArrayInputStream
import java.time.Instant

import cats.effect.IO
import cats.~>
import cats.syntax.compose._
import fluence.crypto.SignAlgo
import fluence.crypto.algorithm.Ecdsa
import fluence.kad.grpc.{KademliaGrpc, LookupAwayRequest, LookupRequest, PingRequest}
import fluence.kad.grpc.server.KademliaServer
import fluence.kad.protocol.{Contact, KademliaRpc, Key, Node}
import fluence.proxy.grpc.WebsocketMessage
import org.scalatest.{Matchers, WordSpec}
import slogging.{LogLevel, LoggerConfig, PrintLoggerFactory}
import cats.instances.try_._
import com.google.protobuf.ByteString
import fluence.codec.PureCodec
import io.grpc.MethodDescriptor
import scalapb.GeneratedMessage
import scodec.bits.ByteVector
import fluence.codec.bits.BitsCodecs._
import fluence.codec.pb.ProtobufCodecs._

import scala.concurrent.Future
import scala.util.{Random, Try}

class ProxyUnaryCallSpec extends WordSpec with Matchers {

  LoggerConfig.factory = PrintLoggerFactory()
  LoggerConfig.level = LogLevel.INFO

  type C = Contact

  val algo: SignAlgo = Ecdsa.signAlgo
  import algo.checkerFn
  import fluence.kad.grpc.KademliaNodeCodec.{pureCodec ⇒ nodeCodec}
  implicit val strVecP: PureCodec[ByteVector, ByteString] =
    PureCodec[ByteVector, Array[Byte]] andThen PureCodec[Array[Byte], ByteString]
  val keyC = PureCodec.codec[Key, ByteVector] andThen strVecP

  def rndString(size: Int = 10): String = Random.nextString(size)

  def rndNode(): Node[C] = {
    val kp = algo.generateKeyPair[Try]().value.get.right.get
    val contact = Contact.buildOwn("", 1, 1L, "123", algo.signer(kp)).value.get.right.get
    Node[C](Key.fromKeyPair.runF[Try](kp).get, Instant.now(), contact)
  }

  "proxy" should {

    val rndPingNode = rndNode()
    val rndLookupNodes = List.fill(5)(rndNode())
    val rndLookupAwayNodes = List.fill(5)(rndNode())

    val RPC = {
      new KademliaRpc[C] {
        override def ping(): IO[Node[C]] = IO(rndPingNode)

        override def lookup(key: Key, numberOfNodes: Int): IO[Seq[Node[C]]] = IO(rndLookupNodes)

        override def lookupAway(key: Key, moveAwayFrom: Key, numberOfNodes: Int): IO[Seq[Node[C]]] =
          IO(rndLookupAwayNodes)
      }
    }

    val service = KademliaGrpc.bindService(new KademliaServer(RPC), scala.concurrent.ExecutionContext.global)

    val inProcessGrpc = InProcessGrpc.build[IO]("in-process", List(service)).unsafeRunSync()

    implicit def runFuture: Future ~> IO = new (Future ~> IO) {
      override def apply[A](fa: Future[A]): IO[A] = IO.fromFuture(IO(fa))
    }

    val proxyGrpc = new ProxyGrpc[IO](inProcessGrpc)

    def generateMessage[Req <: GeneratedMessage, Resp](
      req: Req,
      descriptor: MethodDescriptor[Req, Resp]
    ): WebsocketMessage = {
      val splitted = descriptor.getFullMethodName.split("/").toList

      WebsocketMessage(splitted(0), splitted(1), req.toByteString)
    }

    "work with unary calls" in {

      val pingMessage = generateMessage(PingRequest(), KademliaGrpc.METHOD_PING)

      val pingResp = proxyGrpc
        .handleMessage(pingMessage.service, pingMessage.method, pingMessage.protoMessage.newInput())
        .unsafeRunSync()

      nodeCodec.inverse.runF[Try](fluence.kad.grpc.Node.parseFrom(pingResp)).get.contact shouldBe rndPingNode.contact
      nodeCodec.inverse.runF[Try](fluence.kad.grpc.Node.parseFrom(pingResp)).get.key shouldBe rndPingNode.key

      val key = rndLookupNodes.head.key
      val lookupReq = LookupRequest(keyC.direct.runF[Try](key).get, rndLookupNodes.size)
      val lookupMessage = generateMessage(lookupReq, KademliaGrpc.METHOD_LOOKUP)

      val lookupResp = KademliaGrpc.METHOD_LOOKUP.getResponseMarshaller
        .parse(
          new ByteArrayInputStream(
            proxyGrpc
              .handleMessage(lookupMessage.service, lookupMessage.method, lookupMessage.protoMessage.newInput())
              .unsafeRunSync()
          )
        )
        .nodes
        .map(nodeCodec.inverse.runF[Try])
        .map(_.get)

      lookupResp.map(_.key) shouldBe rndLookupNodes.map(_.key)
      lookupResp.map(_.contact) shouldBe rndLookupNodes.map(_.contact)

      val lookupAwayReq =
        LookupAwayRequest(keyC.direct.runF[Try](key).get, keyC.direct.runF[Try](key).get, rndLookupAwayNodes.size)
      val lookupAwayMessage = generateMessage(lookupAwayReq, KademliaGrpc.METHOD_LOOKUP_AWAY)

      val lookupAwayResp = KademliaGrpc.METHOD_LOOKUP_AWAY.getResponseMarshaller
        .parse(
          new ByteArrayInputStream(
            proxyGrpc
              .handleMessage(
                lookupAwayMessage.service,
                lookupAwayMessage.method,
                lookupAwayMessage.protoMessage.newInput()
              )
              .unsafeRunSync()
          )
        )
        .nodes
        .map(nodeCodec.inverse.runF[Try])
        .map(_.get)

      lookupAwayResp.map(_.key) shouldBe rndLookupAwayNodes.map(_.key)
      lookupAwayResp.map(_.contact) shouldBe rndLookupAwayNodes.map(_.contact)
    }
  }
}
