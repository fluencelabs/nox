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

import java.net.InetAddress
import java.nio.file.Paths

import cats.effect.{ContextShift, IO, Resource, Timer}
import cats.{Applicative, Apply, Functor, Monad, Parallel}
import fluence.Eventually
import fluence.effects.docker.params.{DockerImage, DockerLimits}
import fluence.effects.tendermint.block.TestData
import fluence.effects.tendermint.block.data.{Block, Header}
import fluence.effects.tendermint.rpc.http.{RpcBodyMalformed, RpcError, RpcRequestFailed}
import fluence.log.{Log, LogFactory}
import cats.syntax.apply._
import cats.syntax.applicative._
import fluence.effects.tendermint.block.history.BlockManifest
import fluence.node.config.DockerConfig
import fluence.node.eth.state._
import fluence.node.workers.subscription._
import fluence.node.workers.tendermint.config.{ConfigTemplate, TendermintConfig}
import fluence.node.workers.websocket.WebsocketRequests.{
  LastManifestRequest,
  P2pPortRequest,
  StatusRequest,
  TxRequest,
  TxWaitRequest,
  WebsocketRequest
}
import fluence.node.workers.websocket.WebsocketResponses.{
  ErrorResponse,
  LastManifestResponse,
  P2pPortResponse,
  StatusResponse,
  TxResponse,
  TxWaitResponse,
  WebsocketResponse
}
import fluence.node.workers.websocket.WorkersWebsocket
import fluence.node.workers.{Worker, WorkerApi, WorkerParams}
import fluence.statemachine.data.Tx
import org.scalatest.{BeforeAndAfterAll, Matchers, WordSpec}
import scodec.bits.ByteVector
import io.circe.syntax._
import io.circe.parser.parse

import scala.compat.Platform.currentTime
import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._
import scala.language.higherKinds

class WebsocketApiSpec extends WordSpec with Matchers with BeforeAndAfterAll with Eventually {

  import fluence.node.workers.websocket.WebsocketRequests.WebsocketRequest._
  import fluence.node.workers.websocket.WebsocketResponses.WebsocketResponse._

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val logFactory = LogFactory.forPrintln[IO](level = Log.Error)
  implicit private val log = logFactory.init("ResponseSubscriberSpec", level = Log.Off).unsafeRunSync()

  def websocketApi(workerApi: WorkerApi) = WorkersWebsocket[IO](null, workerApi)

  "Weboscket API" should {

    "return an error if cannot parse a request" in {
      val request = "some incorrect request"
      val response = websocketApi(new TestWorkerApi()).processRequest(request).unsafeRunSync()

      val parsedResponse = parse(response).flatMap(_.as[WebsocketResponse]).right.get.asInstanceOf[ErrorResponse]

      parsedResponse.requestId shouldBe ""
      parsedResponse.error should startWith("Cannot parse msg. Error: io.circe.ParsingFailure")
    }

    "return a p2pPort" in {

      val p2pPortV: Short = 123

      val id = "some-id"
      val request: WebsocketRequest = P2pPortRequest(id)
      val response = websocketApi(new TestWorkerApi {
        override def p2pPort[F[_]: Monad](worker: Worker[F])(implicit log: Log[F]): F[Short] =
          p2pPortV.pure[F]
      }).processRequest(request.asJson.spaces4).unsafeRunSync()
      val parsedResponse = parse(response).flatMap(_.as[WebsocketResponse]).right.get.asInstanceOf[P2pPortResponse]

      parsedResponse.requestId shouldBe id
      parsedResponse.p2pPort shouldBe p2pPortV
    }

    "return a correct status" in {
      val statusV = "some status"
      val id = "some-id"

      val request: WebsocketRequest = StatusRequest(id)
      val response = websocketApi(new TestWorkerApi {
        override def tendermintStatus[F[_]: Monad](
          worker: Worker[F]
        )(implicit log: Log[F]): F[Either[RpcError, String]] =
          (Right(statusV): Either[RpcError, String]).pure[F]
      }).processRequest(request.asJson.spaces4).unsafeRunSync()
      val parsedResponse = parse(response).flatMap(_.as[WebsocketResponse]).right.get.asInstanceOf[StatusResponse]

      parsedResponse.requestId shouldBe id
      parsedResponse.status shouldBe statusV
    }

    "return a correct error on status" in {
      val error = RpcBodyMalformed(new RuntimeException("some error"))
      val id = "some-id"

      val request: WebsocketRequest = StatusRequest(id)
      val response = websocketApi(new TestWorkerApi {
        override def tendermintStatus[F[_]: Monad](
          worker: Worker[F]
        )(implicit log: Log[F]): F[Either[RpcError, String]] =
          (Left(error): Either[RpcError, String]).pure[F]
      }).processRequest(request.asJson.spaces4).unsafeRunSync()
      val parsedResponse = parse(response).flatMap(_.as[WebsocketResponse]).right.get.asInstanceOf[ErrorResponse]

      parsedResponse.requestId shouldBe id
      parsedResponse.error shouldBe error.getMessage
    }

    "return a last manifest" in {
      val bv = ByteVector(1, 2, 3)
      val header = Header(None, "123", 5, None, 10L, 7L, None, bv, bv, bv, bv, bv, bv, bv, bv, bv)
      val manifest = BlockManifest(bv, None, None, header, Nil, Nil)
      val id = "some-id"

      val request: WebsocketRequest = LastManifestRequest(id)
      val response = websocketApi(new TestWorkerApi {
        override def lastManifest[F[_]: Monad](worker: Worker[F]): F[Option[BlockManifest]] =
          Option(manifest).pure[F]
      }).processRequest(request.asJson.spaces4).unsafeRunSync()
      val parsedResponse = parse(response).flatMap(_.as[WebsocketResponse]).right.get.asInstanceOf[LastManifestResponse]

      parsedResponse.requestId shouldBe id
      parsedResponse.lastManifest shouldBe Some(manifest.jsonString)
    }

    "return a transaction response" in {
      val id = "some-id"
      val txResponse = "tx-response"
      val txRequest = "tx-request"

      val request: WebsocketRequest = TxRequest(txRequest, None, id)
      val response = websocketApi(new TestWorkerApi {
        override def sendTx[F[_]: Monad](worker: Worker[F], tx: String, id: Option[String])(
          implicit log: Log[F]
        ): F[Either[RpcError, String]] =
          (Right(tx + txResponse): Either[RpcError, String]).pure[F]
      }).processRequest(request.asJson.spaces4).unsafeRunSync()
      val parsedResponse = parse(response).flatMap(_.as[WebsocketResponse]).right.get.asInstanceOf[TxResponse]

      parsedResponse.requestId shouldBe id
      parsedResponse.data shouldBe txRequest + txResponse
    }

    "return a correct error on transaction" in {
      val id = "some-id"
      val error = RpcBodyMalformed(new RuntimeException("some error"))
      val txRequest = "tx-request"

      val request: WebsocketRequest = TxRequest(txRequest, None, id)
      val response = websocketApi(new TestWorkerApi {
        override def sendTx[F[_]: Monad](worker: Worker[F], tx: String, id: Option[String])(
          implicit log: Log[F]
        ): F[Either[RpcError, String]] =
          (Left(error): Either[RpcError, String]).pure[F]
      }).processRequest(request.asJson.spaces4).unsafeRunSync()
      val parsedResponse = parse(response).flatMap(_.as[WebsocketResponse]).right.get.asInstanceOf[ErrorResponse]

      parsedResponse.requestId shouldBe id
      parsedResponse.error shouldBe error.getMessage
    }

    "return correct responses on transaction await" in {
      val id = "some-id"
      val txRequest = "tx-request"

      def call(request: WebsocketRequest,
               responseApi: Either[TxAwaitError, TendermintQueryResponse]): WebsocketResponse = {
        val response = websocketApi(new TestWorkerApi {
          override def sendTxAwaitResponse[F[_]: Monad, G[_]](worker: Worker[F], tx: String, id: Option[String])(
            implicit log: Log[F]
          ): F[Either[TxAwaitError, TendermintQueryResponse]] =
            responseApi.pure[F]
        }).processRequest(request.asJson.spaces4).unsafeRunSync()
        parse(response).flatMap(_.as[WebsocketResponse]).right.get
      }

      val txResponse = "response"
      val request: WebsocketRequest = TxWaitRequest(txRequest, None, id)
      val head = Tx.Head("session", 1L)

      val responseApi1 = Right(OkResponse(head, txResponse))
      val response1 = call(request, responseApi1).asInstanceOf[TxWaitResponse]

      response1.requestId shouldBe id
      response1.data shouldBe txResponse

      val error2 = RpcBodyMalformed(new RuntimeException("some error"))
      val responseApi2 = Right(RpcErrorResponse(Tx.Head("session", 2L), error2))
      val response2 = call(request, responseApi2).asInstanceOf[ErrorResponse]

      response2.requestId shouldBe id
      response2.error shouldBe error2.getMessage

      val error3 = RpcBodyMalformed(new RuntimeException("some error"))
      val responseApi3 = Left(RpcTxAwaitError(error3))
      val response3 = call(request, responseApi3).asInstanceOf[ErrorResponse]

      response3.requestId shouldBe id
      response3.error shouldBe error3.getMessage

      val timedOut4 = TimedOutResponse(head, 4)
      val responseApi4 = Right(timedOut4)
      val response4 = call(request, responseApi4).asInstanceOf[ErrorResponse]

      response4.requestId shouldBe id
      response4.error shouldBe s"Cannot get response after ${timedOut4.tries} generated blocks"
    }
  }
}
