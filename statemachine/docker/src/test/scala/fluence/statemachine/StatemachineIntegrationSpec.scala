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

import cats.data.NonEmptyList
import cats.effect.{ContextShift, IO, Timer}
import com.github.jtendermint.jabci.types.{RequestCheckTx, RequestCommit, RequestDeliverTx, RequestQuery}
import com.google.protobuf.ByteString
import fluence.effects.sttp.SttpEffect
import fluence.log.{Log, LogFactory}
import fluence.statemachine.abci.AbciHandler
import fluence.statemachine.abci.peers.PeersControlBackend
import fluence.statemachine.api.command.{PeersControl, ReceiptBus}
import fluence.statemachine.api.data.BlockReceipt
import fluence.statemachine.api.query.QueryCode
import fluence.statemachine.api.tx.TxCode
import org.scalatest.{Matchers, OneInstancePerTest, WordSpec}
import scodec.bits.ByteVector

import scala.concurrent.ExecutionContext.Implicits.global

class StatemachineIntegrationSpec extends WordSpec with Matchers with OneInstancePerTest {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)
  implicit val lf: LogFactory[IO] = LogFactory.forPrintln(Log.Error)
  implicit val log: Log[IO] = lf.init(getClass.getSimpleName).unsafeRunSync()
  implicit private val sttp = SttpEffect.stream[IO]

  // sbt defaults user directory to submodule directory
  // while Idea defaults to project root
  private val moduleDirPrefix =
    if (System.getProperty("user.dir").endsWith("/statemachine/docker")) s"${System.getProperty("user.dir")}/../../"
    else s"${System.getProperty("user.dir")}/../"
  private val llamadbPath = moduleDirPrefix + "vm/src/it/resources/llama_db.wasm"
  private val config = StateMachineConfig(
    8,
    llamadbPath :: Nil,
    "OFF",
    26661,
    HttpConfig("localhost", 26657),
    blockUploadingEnabled = true
  )

  val peersBackend = PeersControlBackend[IO].unsafeRunSync()

  val machine = EmbeddedStateMachine
    .init[IO](
      NonEmptyList.one(llamadbPath),
      config.blockUploadingEnabled
    )
    .map(_.extend[PeersControl[IO]](peersBackend))
    .value
    .flatMap(
      e ⇒ IO.fromEither(e.left.map(err ⇒ new RuntimeException(s"Cannot initiate EmbeddedStateMachine: $err")))
    )
    .unsafeRunSync()

  val abciHandler: AbciHandler[IO] = AbciHandler(machine, peersBackend)

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
      case code if code == TxCode.OK.id => Right(new String(response.getValue.toByteArray))
      case _                            => Left((response.getCode, response.getInfo))
    }
  }

  def uploadReceipt(height: Long) =
    machine.command[ReceiptBus[IO]].sendBlockReceipt(BlockReceipt(height, ByteVector.empty)).value.unsafeRunSync()

  def tx(session: String, order: Long, payload: String): String =
    s"$session/$order\n$payload"

  "State machine" should {
    val session = "157A0E"
    val tx0 = tx(
      session,
      0,
      "CREATE TABLE Users(id INT, name TEXT, age INT)"
    )
    val tx1 = tx(
      session,
      1,
      "INSERT INTO Users VALUES(1, 'Monad', 23)," +
        "(2, 'Applicative Functor', 19)," +
        "(3, 'Free Monad', 31)," +
        "(4, 'Tagless Final', 25)"
    )
    val tx2 = tx(session, 2, "SELECT COUNT(*) FROM Users")
    val tx3 = tx(session, 3, "SELECT min(id), max(id), count(age), sum(age), avg(age) FROM Users")
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

      uploadReceipt(0)
      uploadReceipt(1)
      uploadReceipt(2)
      uploadReceipt(3)

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

      sendQuery(tx1Result) shouldBe Right("rows inserted: 4")
      sendQuery(tx3Result) shouldBe Right("_0, _1, _2, _3, _4\n1, 4, 4, 98, 24.5")

//      latestCommittedHeight shouldBe 5
//      latestAppHash shouldBe "fbca0d73019bc3ac6c8960782fe681835c13ace92a0d1dffd73fd363a173122c"
    }

    "invoke session txs in session counter order" in {
      sendCommit()
      sendCommit()

      sendDeliverTx(tx0)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)

      uploadReceipt(1)
      uploadReceipt(2)

      sendCommit()
      sendCommit()

      sendQuery(tx1Result) shouldBe Right("rows inserted: 4")
      sendQuery(tx1Result).left.get._1 shouldBe QueryCode.Pending.id
      sendQuery(tx2Result).left.get._1 shouldBe QueryCode.Pending.id
      sendQuery(tx3Result).left.get._1 shouldBe QueryCode.Pending.id

      sendDeliverTx(tx1)

      uploadReceipt(3)
      uploadReceipt(4)

      sendCommit()
      sendCommit()

      sendQuery(tx0Result) shouldBe Right("table created")
      sendQuery(tx1Result) shouldBe Right("rows inserted: 4")
      sendQuery(tx2Result) shouldBe Right("4")
      sendQuery(tx3Result) shouldBe Right("\"_0, _1, _2, _3, _4\\n1, 4, 4, 98, 24.5\"")

    }

    "ignore duplicated tx" in {
      sendCommit()
      sendCommit()

      sendCheckTx(tx0)._1 shouldBe TxCode.OK.id
      sendDeliverTx(tx0)._1 shouldBe TxCode.OK.id
      // Mempool state updated only on commit!
      sendCheckTx(tx0)._1 shouldBe TxCode.OK.id

      uploadReceipt(1)
      uploadReceipt(2)

      sendCommit()

      sendCheckTx(tx0)._1 shouldBe TxCode.AlreadyProcessed.id
      sendDeliverTx(tx0)._1 shouldBe TxCode.AlreadyProcessed.id
    }

    "process Query method correctly" in {
      sendDeliverTx(tx0)
//      sendQuery(tx0Result) shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.QueryStateIsNotReadyYet))

      uploadReceipt(0)
      uploadReceipt(1)

      sendCommit()
//      sendQuery(tx0Result) shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.QueryStateIsNotReadyYet))

      sendQuery(tx0Result) shouldBe Right("table created")
    }
  }
}
