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
import cats.{effect, Parallel}
import fluence.effects.docker.params.{DockerImage, DockerLimits}
import fluence.effects.tendermint.rpc.http.{RpcBodyMalformed, RpcRequestFailed}
import fluence.log.{Log, LogFactory}
import fluence.node.config.DockerConfig
import fluence.node.eth.state._
import fluence.node.workers.WorkersApi.{RpcTxAwaitError, TxAwaitError, TxParsingError}
import fluence.node.workers.subscription._
import fluence.node.workers.tendermint.config.{ConfigTemplate, TendermintConfig}
import fluence.node.workers.{WorkerParams, WorkersApi, WorkersPool}
import org.scalatest.{BeforeAndAfterAll, Matchers, WordSpec}
import scodec.bits.ByteVector

import scala.compat.Platform.currentTime
import scala.concurrent.ExecutionContext.Implicits.global
import scala.concurrent.duration._

class ResponseSubscriberSpec extends WordSpec with Matchers with BeforeAndAfterAll {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit private val logFactory = LogFactory.forPrintln[IO](level = Log.Trace)
  implicit private val log = logFactory.init("RequestResponseSpec", level = Log.Trace).unsafeRunSync()

  def start() = {
    val rootPath = Paths.get("/tmp")

    val appId = 1L
    val p2pPort = 10001.toShort
    val workerPeer = WorkerPeer(ByteVector.empty, "", 25000.toShort, InetAddress.getLocalHost, 0)
    val cluster = Cluster(currentTime.millis, Vector.empty, workerPeer)
    val app = App(appId, StorageRef(ByteVector.empty, StorageType.Ipfs), cluster)
    val dockerConfig = DockerConfig(DockerImage("fluencelabs/worker", "v0.2.0"), DockerLimits(None, None, None))
    val tmDockerConfig = DockerConfig(DockerImage("tendermint/tendermint", "v0.32.0"), DockerLimits(None, None, None))
    val tmConfig = TendermintConfig("info", 0, 0, 0, 0L, false, false, false, p2pPort, Seq.empty)
    val configTemplate = ConfigTemplate[IO](rootPath, tmConfig).unsafeRunSync()
    val params = WorkerParams(app, rootPath, rootPath, None, dockerConfig, tmDockerConfig, configTemplate)

    for {
      tendermint <- Resource.liftF(TendermintTest[IO]())
      requestResponder <- Resource
        .liftF[IO, ResponseSubscriberImpl[IO, effect.IO.Par]](
          ResponseSubscriberImpl[IO, IO.Par](tendermint.tendermint, appId)
        )
      pool <- Resource.liftF(TestWorkersPool.some[IO](requestResponder, tendermint.tendermint))
      _ <- Resource.liftF(pool.run(appId, IO(params)))
      _ <- requestResponder.start()
    } yield (pool, requestResponder, tendermint)
  }

  def tx(nonce: Int) =
    s"""|asdf/$nonce
        |this_should_be_a_llamadb_signature_but_it_doesnt_matter_for_this_test
        |1
        |INSERT INTO users VALUES(1, 'Sara', 23), (2, 'Bob', 19), (3, 'Caroline', 31), (4, 'Max', 27)
        |""".stripMargin

  def txResponse(code: Int) =
    s"""
       |{
       |
       |
       |    "error": "",
       |    "result": {
       |        "hash": "2B8EC32BA2579B3B8606E42C06DE2F7AFA2556EF",
       |        "log": "",
       |        "data": "",
       |        "code": "$code"
       |    },
       |    "id": "",
       |    "jsonrpc": "2.0"
       |
       |}
       |""".stripMargin

  private val correctTxResponse = txResponse(0)

  private def queryResponse(code: Int) =
    s"""
       |{
       |
       |
       |    "error": "",
       |    "result": {
       |        "response": {
       |            "log": "exists",
       |            "height": "0",
       |            "value": "61626364",
       |            "key": "61626364",
       |            "index": "-1",
       |            "code": "$code"
       |        }
       |    },
       |    "id": "",
       |    "jsonrpc": "2.0"
       |
       |}
       |""".stripMargin

  private val correctQueryResponse = queryResponse(0)
  private val pendingQueryResponse = queryResponse(3)

  def request(pool: WorkersPool[IO], requestSubscriber: ResponseSubscriber[IO], txCustom: Option[String] = None)(
    implicit P: Parallel[IO, IO.Par],
    log: Log[IO]
  ): IO[Either[TxAwaitError, TendermintQueryResponse]] = requests(1, pool, requestSubscriber, txCustom).map(_.head)

  def requests(to: Int,
               pool: WorkersPool[IO],
               requestSubscriber: ResponseSubscriber[IO],
               txCustom: Option[String] = None)(
    implicit P: Parallel[IO, IO.Par],
    log: Log[IO]
  ): IO[List[Either[TxAwaitError, TendermintQueryResponse]]] = {
    import cats.instances.list._
    import cats.syntax.parallel._

    Range(0, to).toList.map { nonce =>
      WorkersApi.txAwaitResponse[IO, IO.Par](pool, 1, txCustom.getOrElse(tx(nonce)), None)
    }.parSequence
  }

  "MasterNode API" should {
    "return an RPC error, if broadcastTx returns an error" in {

      val result = start().use {
        case (pool, requestSubscriber, _) =>
          for {
            response <- request(pool, requestSubscriber)
          } yield response
      }.unsafeRunSync()

      result should be('left)
      result.left.get shouldBe a[RpcTxAwaitError]

      val error = result.left.get.asInstanceOf[RpcTxAwaitError]
      error.rpcError shouldBe a[RpcRequestFailed]
    }

    "return an malformed error if tx response from tendermint is incorrect" in {

      val result = start().use {
        case (pool, requestSubscriber, tendermintTest) =>
          for {
            _ <- tendermintTest.setTxResponse(Right(""))
            response <- request(pool, requestSubscriber)
          } yield response
      }.unsafeRunSync()

      result should be('left)
      result.left.get shouldBe a[RpcTxAwaitError]

      val error = result.left.get.asInstanceOf[RpcTxAwaitError]
      error.rpcError shouldBe a[RpcBodyMalformed]
    }

    "return an error if tx is incorrect" in {

      val result = start().use {
        case (pool, requestSubscriber, tendermintTest) =>
          for {
            response <- request(pool, requestSubscriber, Some("failed"))
          } yield response
      }.unsafeRunSync()

      result should be('left)
      result.left.get shouldBe a[TxParsingError]
    }

    "return an error if query API from tendermint is not responded" in {

      val result = start().use {
        case (pool, requestSubscriber, tendermintTest) =>
          for {
            _ <- tendermintTest.setTxResponse(Right(correctTxResponse))
            response <- request(pool, requestSubscriber)
          } yield response
      }.unsafeRunSync()

      result should be('right)
      result.right.get shouldBe a[RpcErrorResponse]

      val error = result.right.get.asInstanceOf[RpcErrorResponse]
      error.error shouldBe a[RpcRequestFailed]
    }

    "return an error if query API returns incorrect response" in {

      val result = start().use {
        case (pool, requestSubscriber, tendermintTest) =>
          for {
            _ <- tendermintTest.setTxResponse(Right(correctTxResponse))
            _ <- tendermintTest.setQueryResponse(Right("incorrectTxResponse"))
            response <- request(pool, requestSubscriber)
          } yield response
      }.unsafeRunSync()

      result should be('right)
      result.right.get shouldBe a[RpcErrorResponse]

      val error = result.right.get.asInstanceOf[RpcErrorResponse]
      error.error shouldBe a[RpcBodyMalformed]
    }

    "return a pending response, if tendermint cannot return response after some amount of blocks" in {

      val result = start().use {
        case (pool, requestSubscriber, tendermintTest) =>
          for {
            _ <- tendermintTest.setTxResponse(Right(correctTxResponse))
            _ <- tendermintTest.setQueryResponse(Right(pendingQueryResponse))
            response <- request(pool, requestSubscriber)
          } yield response
      }.unsafeRunSync()

      result should be('right)
      result.right.get shouldBe a[TimedOutResponse]

      val error = result.right.get.asInstanceOf[TimedOutResponse]
      error.body shouldBe pendingQueryResponse
    }

    "return OK result if tendermint is responded ok" in {
      val result = start().use {
        case (pool, requestSubscriber, tendermintTest) =>
          for {
            _ <- tendermintTest.setTxResponse(Right(correctTxResponse))
            _ <- tendermintTest.setQueryResponse(Right(correctQueryResponse))
            response <- request(pool, requestSubscriber)
          } yield response
      }.unsafeRunSync()

      result should be('right)
      result.right.get shouldBe a[OkResponse]

      val ok = result.right.get.asInstanceOf[OkResponse]
      ok.body shouldBe correctQueryResponse
    }
  }
}
