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

package fluence.grpc.proxy

import com.google.protobuf.ByteString
import fluence.grpc.proxy.test.TestServiceGrpc.TestService
import fluence.grpc.proxy.test.{TestMessage, TestRequest, TestResponse, TestServiceGrpc}
import fluence.proxy.grpc.WebsocketMessage
import io.grpc.stub.StreamObserver
import io.grpc.{MethodDescriptor, ServerServiceDefinition}
import monix.eval.Task
import monix.execution.Scheduler.Implicits.global
import monix.reactive.Observable
import org.scalatest.{Matchers, WordSpec}
import scalapb.GeneratedMessage

import scala.concurrent.duration._
import scala.concurrent.Future
import scala.util.Random

class ProxyCallSpec extends WordSpec with Matchers with slogging.LazyLogging {

  "proxy" should {

    val respChecker = "resp"
    val respCheckerBytes = ByteString.copyFrom(Array[Byte](5, 6, 7, 8))

    val RPC = {
      new TestService {
        override def test(request: TestRequest): Future[TestResponse] = {
          val resp = request.message.map { r ⇒
            r.copy(str = r.str + respChecker, listStr = r.listStr :+ respChecker, r.byteArray.concat(respCheckerBytes))
          }
          Future(TestResponse(resp))
        }

        override def testCount(responseObserver: StreamObserver[TestResponse]): StreamObserver[TestRequest] = {
          new StreamObserver[TestRequest] {
            override def onNext(value: TestRequest): Unit = {
              if (!value.close) {
                val resp = TestResponse(value.message.map(m ⇒ m.copy(counter = m.counter + 1)))
                responseObserver.onNext(resp)
              } else {
                responseObserver.onCompleted()
              }
            }

            override def onError(t: Throwable): Unit = {
              t.printStackTrace()
            }

            override def onCompleted(): Unit = ()
          }
        }
      }
    }

    val service = TestServiceGrpc.bindService(RPC, scala.concurrent.ExecutionContext.global)

    def inService(services: List[ServerServiceDefinition] = List(service))(use: InProcessGrpc ⇒ Any) =
      Task
        .fromIO(InProcessGrpc.build("in-process", services))
        .bracket(ipg ⇒ Task.eval(use(ipg)))(ipg ⇒ Task.fromIO(ipg.unsafeClose()))
        .runSyncUnsafe(15.seconds)

    def generateMessage[Req <: GeneratedMessage, Resp](
      streamId: Long,
      req: Req,
      descriptor: MethodDescriptor[Req, Resp]
    ): WebsocketMessage = {
      val splitted = descriptor.getFullMethodName.split("/").toList

      WebsocketMessage(splitted(0), splitted(1), streamId, req.toByteString)
    }

    def message(streamId: Long, c: Int, close: Boolean = false): WebsocketMessage =
      generateMessage(
        streamId,
        TestRequest(Some(TestMessage("", List(), ByteString.copyFrom(Array[Byte]()), c)), close = close),
        TestServiceGrpc.METHOD_TEST_COUNT
      )

    def sendMessage(proxyGrpc: ProxyGrpc, message: WebsocketMessage): Observable[Array[Byte]] = {

      proxyGrpc
        .handleMessage(
          message.service,
          message.method,
          message.streamId,
          message.payload.newInput()
        )
        .runSyncUnsafe(5.seconds)
    }

    "work with unary calls" in {
      inService() { inProcessGrpc ⇒
        val proxyGrpc = new ProxyGrpc(inProcessGrpc)

        val str = "test"
        val listStr = Seq("test1", "test2")
        val byteArray = ByteString.copyFrom(Array[Byte](1, 2, 3, 4, 5))

        val streamId = Random.nextLong()

        val testMessage =
          generateMessage(
            streamId,
            TestRequest(Some(TestMessage(str, listStr, byteArray))),
            TestServiceGrpc.METHOD_TEST
          )

        val testResp = sendMessage(proxyGrpc, testMessage).lastL.runSyncUnsafe(5.seconds)

        val respBytes = testResp

        val resp = TestRequest.parseFrom(respBytes).message.get
        resp.str shouldBe str + "resp"
        resp.listStr shouldBe listStr :+ "resp"
        resp.byteArray shouldBe byteArray.concat(respCheckerBytes)
      }
    }

    "work with bidi streams" in {

      inService() { inProcessGrpc ⇒
        val proxyGrpc = new ProxyGrpc(inProcessGrpc)

        val streamId = Random.nextLong()

        val results = Range(1, 10).toList
        val lastResult = results.last

        val responseObservable =
          sendMessage(proxyGrpc, message(streamId, 0)).map(bytes ⇒ TestRequest.parseFrom(bytes).message.get.counter)

        //imitate client that do request on every response
        responseObservable.flatMap {
          case counter if counter == lastResult ⇒
            val msgClose = message(streamId, results.last, close = true)
            sendMessage(proxyGrpc, msgClose)
          case counter ⇒
            val testMessage = message(streamId, counter)
            sendMessage(proxyGrpc, testMessage)
        }.subscribe()

        responseObservable.foldLeftL(0)(_ + _).runSyncUnsafe(5.seconds) shouldBe results.sum
      }

    }

    "raise error if the proxy was closed" in {
      inService() { inProcessGrpc ⇒
        val proxyGrpc = new ProxyGrpc(inProcessGrpc)

        inProcessGrpc.close().unsafeRunSync()

        val testMessage =
          generateMessage(1111L, TestRequest(Some(TestMessage())), TestServiceGrpc.METHOD_TEST)

        the[RuntimeException] thrownBy {
          proxyGrpc
            .handleMessage(testMessage.service, testMessage.method, 1L, testMessage.payload.newInput())
            .runSyncUnsafe(5.seconds)
            .lastL
            .runSyncUnsafe(5.seconds)
        }
      }
    }

    "raise error if no method or service descriptor in proxy" in {
      inService() { inProcessGrpc ⇒
        val proxyGrpc = new ProxyGrpc(inProcessGrpc)

        val testMessage =
          generateMessage(555L, TestRequest(Some(TestMessage())), TestServiceGrpc.METHOD_TEST)

        the[RuntimeException] thrownBy {
          proxyGrpc
            .handleMessage("rndservice", testMessage.method, 1L, testMessage.payload.newInput())
            .runSyncUnsafe(5.seconds)
        }

        the[RuntimeException] thrownBy {
          proxyGrpc
            .handleMessage(testMessage.service, "rndmethod", 1L, testMessage.payload.newInput())
            .runSyncUnsafe(5.seconds)
        }
      }
    }
  }
}
