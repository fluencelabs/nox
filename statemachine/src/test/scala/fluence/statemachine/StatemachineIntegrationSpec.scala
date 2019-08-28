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

package fluence.statemachine

import java.nio.ByteBuffer

import cats.data.EitherT
import cats.effect.{ContextShift, IO, Timer}
import com.github.jtendermint.jabci.types.{RequestCheckTx, RequestCommit, RequestDeliverTx, RequestQuery}
import com.google.protobuf.ByteString
import com.softwaremill.sttp.SttpBackend
import fluence.EitherTSttpBackend
import fluence.effects.tendermint.rpc.http.{TendermintHttpRpc, TendermintHttpRpcImpl}
import fluence.log.{Log, LogFactory}
import fluence.statemachine.config.{StateMachineConfig, TendermintRpcConfig}
import fluence.statemachine.control.ControlServer
import fluence.statemachine.control.signals.{ControlSignals, MockedControlSignals}
import fluence.statemachine.data.{QueryCode, TxCode}
import org.scalatest.{Matchers, OneInstancePerTest, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global

class StatemachineIntegrationSpec extends WordSpec with Matchers with OneInstancePerTest {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit val lf: LogFactory[IO] = LogFactory.forPrintln(Log.Error)
  implicit val log: Log[IO] = lf.init(getClass.getSimpleName).unsafeRunSync()
  implicit private val sttp: SttpBackend[EitherT[IO, Throwable, ?], fs2.Stream[IO, ByteBuffer]] =
    EitherTSttpBackend[IO]()

  // sbt defaults user directory to submodule directory
  // while Idea defaults to project root
  private val moduleDirPrefix = if (System.getProperty("user.dir").endsWith("/statemachine")) "../" else "./"
  private val moduleFiles = List("mul.wast", "counter.wast").map(moduleDirPrefix + "vm/src/test/resources/wast/" + _)
  private val config = StateMachineConfig(
    8,
    moduleFiles,
    "OFF",
    26661,
    ControlServer.Config("localhost", 26662),
    TendermintRpcConfig("localhost", 26657),
    blockUploadingEnabled = true
  )
  private val signals: ControlSignals[IO] = new MockedControlSignals

  val abciHandler: AbciHandler[IO] = ServerRunner
    .buildAbciHandler(config, signals)
    .valueOr(e => throw new RuntimeException(e.message))
    .unsafeRunSync()

  def sendCheckTx(tx: String): (Int, String) = {
    val request = RequestCheckTx.newBuilder().setTx(ByteString.copyFromUtf8(tx)).build()
    val response = abciHandler.requestCheckTx(request)
    (response.getCode, response.getInfo)
  }

  def sendDeliverTx(tx: String): (Int, String) = {
    val request = RequestDeliverTx.newBuilder().setTx(ByteString.copyFromUtf8(tx)).build()
    val response = abciHandler.receivedDeliverTx(request)
    (response.getCode, response.getInfo)
  }

  def sendCommit(): Unit =
    abciHandler.requestCommit(RequestCommit.newBuilder().build())

  def sendQuery(query: String, height: Int = 0): Either[(Int, String), String] = {
    val builtQuery = RequestQuery.newBuilder().setHeight(height).setPath(query).setProve(false).build()
    val response = abciHandler.requestQuery(builtQuery)
    response.getCode match {
      case code if code == TxCode.OK.id => Right(ByteVector(response.getValue.toByteArray).toHex)
      case _                            => Left((response.getCode, response.getInfo))
    }
  }

  def tx(session: String, order: Long, payload: String): String =
    s"$session/$order\n$payload"

  def littleEndian4ByteHex(number: Int): String =
    Integer.toString(number, 16).reverse.padTo(8, '0').grouped(2).map(_.reverse).mkString.toUpperCase

  "State machine" should {
    val session = "157A0E"
    val tx0 = tx(
      session,
      0,
      "()"
    )
    val tx1 = tx(
      session,
      1,
      "()"
    )
    val tx2 = tx(session, 2, "()")
    val tx3 = tx(session, 3, "()")
    val tx0Result = s"$session/0"
    val tx1Result = s"$session/1"
    val tx2Result = s"$session/2"
    val tx3Result = s"$session/3"

    "process correct tx/query sequence" in {
      // TODO: rewrite tests. 2 kinds of tests required:
      // 1. Many VM-agnostic tests for State machine logic would not depend on currently used VM impl (with VM stubbed).
      // 2. Few VM-SM tests (similar to the current one) integration tests that fix actual app_hash'es for integration.
      //
      // Currently only this test looks like integrational, other tests should be more decoupled from VM logic.

      sendCommit()
      sendCommit()
//      latestAppHash shouldBe "42bb448ea02f6f4fe069f89e392315602f5463d223cbd0a8246ac42c521ea6bb"

      sendCheckTx(tx0)
      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result).left.get._1 shouldBe QueryCode.NotFound.id
      sendDeliverTx(tx0)
      sendCommit()
//      latestAppHash shouldBe "7b0a908531e5936acdfce3c581ba6b39c2ca185553f47b167440490b13bfa132"

      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result).left.get._1 shouldBe QueryCode.Pending.id
      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
//      latestAppHash shouldBe "fbca0d73019bc3ac6c8960782fe681835c13ace92a0d1dffd73fd363a173122c"

      sendQuery(tx1Result) shouldBe Right(littleEndian4ByteHex(2))
      sendQuery(tx3Result) shouldBe Right(littleEndian4ByteHex(4))

//      latestCommittedHeight shouldBe 5
//      latestAppHash shouldBe "fbca0d73019bc3ac6c8960782fe681835c13ace92a0d1dffd73fd363a173122c"
    }

    "invoke session txs in session counter order" in {
      sendCommit()
      sendCommit()

      sendDeliverTx(tx0)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      sendCommit()

      sendQuery(tx0Result) shouldBe Right(littleEndian4ByteHex(1))
      sendQuery(tx1Result).left.get._1 shouldBe QueryCode.Pending.id
      sendQuery(tx2Result).left.get._1 shouldBe QueryCode.Pending.id
      sendQuery(tx3Result).left.get._1 shouldBe QueryCode.Pending.id

      sendDeliverTx(tx1)
      sendCommit()
      sendCommit()

      sendQuery(tx0Result) shouldBe Right(littleEndian4ByteHex(1))
      sendQuery(tx1Result) shouldBe Right(littleEndian4ByteHex(2))
      sendQuery(tx2Result) shouldBe Right(littleEndian4ByteHex(3))
      sendQuery(tx3Result) shouldBe Right(littleEndian4ByteHex(4))
    }

    "ignore duplicated tx" in {
      sendCommit()
      sendCommit()

      sendCheckTx(tx0)._1 shouldBe TxCode.OK.id
      sendDeliverTx(tx0)._1 shouldBe TxCode.OK.id
      // Mempool state updated only on commit!
      sendCheckTx(tx0)._1 shouldBe TxCode.OK.id
      sendCommit()

      sendCheckTx(tx0)._1 shouldBe TxCode.AlreadyProcessed.id
      sendDeliverTx(tx0)._1 shouldBe TxCode.AlreadyProcessed.id
    }

    "process Query method correctly" in {
      sendDeliverTx(tx0)
//      sendQuery(tx0Result) shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.QueryStateIsNotReadyYet))

      sendCommit()
//      sendQuery(tx0Result) shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.QueryStateIsNotReadyYet))

      sendQuery(tx0Result) shouldBe Right(littleEndian4ByteHex(1))
    }
  }
}
