/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package fluence.node

import cats.effect.{ContextShift, IO, Timer}
import cats.syntax.applicative._
import cats.syntax.either._
import cats.syntax.flatMap._
import fluence.bp.tx.{TxCode, TxResponse}
import fluence.effects.tendermint.rpc.http.RpcError
import fluence.effects.testkit.Timed
import fluence.log.{Log, LogFactory}
import fluence.node.workers.api.WorkerApi
import fluence.node.workers.api.websocket.WebsocketRequests.{TxRequest, WebsocketRequest}
import fluence.node.workers.api.websocket.WebsocketResponses.{ErrorResponse, WebsocketResponse}
import fluence.node.workers.api.websocket.WorkerWebsocket
import io.circe.parser.parse
import io.circe.syntax._
import org.scalatest.{BeforeAndAfterAll, Matchers, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global
import scala.language.higherKinds

class WebsocketApiSpec extends WordSpec with Matchers with BeforeAndAfterAll with Timed {
  import fluence.bp
  import fluence.node.workers.api.websocket.{WebsocketResponses ⇒ WSR}
  import WSR.WebsocketResponse._

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val logFactory = LogFactory.forPrintln[IO](level = Log.Error)
  implicit private val log = logFactory.init("ResponseSubscriberSpec", level = Log.Off).unsafeRunSync()

  def websocketApi(workerApi: WorkerApi[IO]) = WorkerWebsocket[IO](workerApi)

  "Weboscket API" should {
    "return an error if cannot parse a request" in {
      val request = "some incorrect request"
      val response = websocketApi(new TestWorkerApi[IO]()).unsafeRunSync().processRequest(request).unsafeRunSync()

      val parsedResponse = parse(response).flatMap(_.as[WebsocketResponse]).right.get.asInstanceOf[ErrorResponse]

      parsedResponse.requestId shouldBe ""
      parsedResponse.error should startWith("Cannot parse msg. Error: io.circe.ParsingFailure")
    }

    "return a transaction response" in {

      val id = "some-id"
      val txResponse = TxResponse(TxCode.OK, "tx-response")
      val txRequest = "tx-request".getBytes

      val request = TxRequest(txRequest, id): WebsocketRequest
      val response = websocketApi(new TestWorkerApi[IO] {

        override def sendTx(tx: Array[Byte])(implicit log: Log[IO]): IO[Either[RpcError, bp.tx.TxResponse]] =
          txResponse.asRight.pure[IO]

      }).unsafeRunSync().processRequest(request.asJson.spaces4).unsafeRunSync()

      val parsedResponse = parse(response)
        .flatMap(_.as[WSR.TxResponse])
        .flatMap(r ⇒ parse(r.data))
        .flatMap(_.as[bp.tx.TxResponse])
        .right
        .get

      parsedResponse.info shouldBe txResponse.info
    }

//    "return a correct error on transaction" in {
//      val id = "some-id"
//      val error = RpcBodyMalformed(new RuntimeException("some error"))
//      val txRequest = "tx-request"
//
//      val request: WebsocketRequest = TxRequest(txRequest, None, id)
//      val response = websocketApi(new TestWorkerApi[IO] {
//        override def sendTx(tx: String, id: Option[String])(
//          implicit log: Log[IO]
//        ): IO[Either[RpcError, String]] =
//          (Left(error): Either[RpcError, String]).pure[IO]
//      }).unsafeRunSync().processRequest(request.asJson.spaces4).unsafeRunSync()
//      val parsedResponse = parse(response).flatMap(_.as[WebsocketResponse]).right.get.asInstanceOf[ErrorResponse]
//
//      parsedResponse.requestId shouldBe id
//      parsedResponse.error shouldBe error.getMessage
//    }
//
//    "return correct responses on transaction await" in {
//      val id = "some-id"
//      val txRequest = "tx-request"
//
//      def call(
//        request: WebsocketRequest,
//        responseApi: Either[TxAwaitError, AwaitedResponse]
//      ): WebsocketResponse = {
//        val response = websocketApi(new TestWorkerApi[IO] {
//          override def sendTxAwaitResponse(tx: String, id: Option[String])(
//            implicit log: Log[IO]
//          ): IO[Either[TxAwaitError, AwaitedResponse]] =
//            responseApi.pure[IO]
//        }).unsafeRunSync().processRequest(request.asJson.spaces4).unsafeRunSync()
//        parse(response).flatMap(_.as[WebsocketResponse]).right.get
//      }
//
//      val txResponse = "response"
//      val request: WebsocketRequest = TxWaitRequest(txRequest, None, id)
//      val head = Tx.Head("session", 1L)
//
//      val responseApi1 = Right(OkResponse(head, txResponse))
//      val response1 = call(request, responseApi1).asInstanceOf[TxWaitResponse]
//
//      response1.requestId shouldBe id
//      response1.data shouldBe txResponse
//
//      val error2 = RpcBodyMalformed(new RuntimeException("some error"))
//      val responseApi2 = Right(RpcErrorResponse(Tx.Head("session", 2L), error2))
//      val response2 = call(request, responseApi2).asInstanceOf[ErrorResponse]
//
//      response2.requestId shouldBe id
//      response2.error shouldBe error2.getMessage
//
//      val error3 = RpcBodyMalformed(new RuntimeException("some error"))
//      val responseApi3 = Left(RpcTxAwaitError(error3))
//      val response3 = call(request, responseApi3).asInstanceOf[ErrorResponse]
//
//      response3.requestId shouldBe id
//      response3.error shouldBe error3.getMessage
//
//      val timedOut4 = TimedOutResponse(head, 4)
//      val responseApi4 = Right(timedOut4)
//      val response4 = call(request, responseApi4).asInstanceOf[ErrorResponse]
//
//      response4.requestId shouldBe id
//      response4.error shouldBe s"Cannot get response after ${timedOut4.tries} generated blocks"
//    }
//
//    "return stream with responses on subscribtion" in {
//      val subscriptionId = "some-id"
//      val requestId = "request-id"
//      val tx = "some-tx"
//
//      val request: WebsocketRequest = SubscribeRequest(requestId, subscriptionId, tx)
//
//      val streamResponse = OkResponse(Tx.Head("sess", 0), s"response ")
//
//      val api = websocketApi(new TestWorkerApi[IO] {
//        override def subscribe(key: WorkerWebsocket.SubscriptionKey, tx: Tx.Data)(
//          implicit log: Log[IO]
//        ): IO[fs2.Stream[IO, TendermintResponse]] =
//          IO(
//            fs2.Stream
//              .awakeEvery[IO](100.millis)
//              .map(t => Right(streamResponse): TendermintResponse)
//          )
//      }).unsafeRunSync()
//
//      val response = api.processRequest(request.asJson.spaces4).unsafeRunSync()
//
//      val parsedResponse = parse(response).flatMap(_.as[WebsocketResponse]).right.get.asInstanceOf[SubscribeResponse]
//
//      parsedResponse.requestId shouldBe requestId
//
//      import fluence.node.workers.api.websocket.WebsocketResponses.WebsocketResponse._
//
//      for {
//        streamEventChecker <- Ref.of[IO, String]("")
//        streamFinalizeChecker <- Ref.of[IO, Boolean](false)
//
//        _ = api.subscriptionEventStream
//          .evalTap(e => streamEventChecker.set(e))
//          .onFinalize(streamFinalizeChecker.set(true))
//          .drain
//          .compile
//          .toList
//          .unsafeRunAsyncAndForget()
//
//        _ <- eventually[IO]({
//          streamEventChecker.get.map(
//            _ shouldBe (TxWaitResponse("", streamResponse.response): WebsocketResponse).asJson.noSpaces
//          )
//        }, 100.millis)
//
//        _ <- api.closeWebsocket()
//
//        _ <- eventually[IO]({
//          streamFinalizeChecker.get.map(_ shouldBe true)
//        }, 100.millis)
//      } yield {}
//
//    }
  }
}
