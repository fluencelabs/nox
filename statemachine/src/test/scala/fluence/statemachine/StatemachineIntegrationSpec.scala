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

import cats.effect.{ContextShift, IO, Timer}
import com.github.jtendermint.jabci.api.CodeType
import com.github.jtendermint.jabci.types.{RequestCheckTx, RequestCommit, RequestDeliverTx, RequestQuery}
import com.google.protobuf.ByteString
import fluence.statemachine.config.StateMachineConfig
import fluence.statemachine.control.ControlServer.ControlServerConfig
import fluence.statemachine.control.ControlSignals
import fluence.statemachine.state.QueryCodeType
import fluence.statemachine.tree.MerkleTreeNode
import fluence.statemachine.tx.Computed
import fluence.statemachine.util.ClientInfoMessages
import org.scalatest.{Matchers, OneInstancePerTest, WordSpec}

import scala.concurrent.ExecutionContext.Implicits.global

class StatemachineIntegrationSpec extends WordSpec with Matchers with OneInstancePerTest {

  implicit private val ioTimer: Timer[IO] = IO.timer(global)
  implicit private val ioShift: ContextShift[IO] = IO.contextShift(global)

  // sbt defaults user directory to submodule directory
  // while Idea defaults to project root
  private val moduleDirPrefix = if (System.getProperty("user.dir").endsWith("/statemachine")) "../" else "./"
  private val moduleFiles = List("mul.wast", "counter.wast").map(moduleDirPrefix + "vm/src/test/resources/wast/" + _)
  private val config = StateMachineConfig(8, moduleFiles, "OFF", 26661, ControlServerConfig("localhost", 26662))

  private val signals: ControlSignals[IO] = ControlSignals[IO]().allocated.unsafeRunSync()._1

  val abciHandler: AbciHandler = ServerRunner
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
    val builtQuery = RequestQuery.newBuilder().setHeight(height).setPath(query).setProve(true).build()
    val response = abciHandler.requestQuery(builtQuery)
    response.getCode match {
      case CodeType.OK => Right(response.getValue.toStringUtf8)
      case _ => Left((response.getCode, response.getInfo))
    }
  }

  def latestCommittedHeight: Long = abciHandler.committer.stateHolder.latestCommittedHeight.unsafeRunSync()

  def latestCommittedState: MerkleTreeNode = abciHandler.committer.stateHolder.mempoolState.unsafeRunSync()

  def latestAppHash: String = latestCommittedState.merkleHash.value.toHex

  def tx(session: String, order: Long, payload: String): String = {
    val txHeaderJson = s"""{"session":"$session","order":$order}"""
    s"""{"header":$txHeaderJson,"payload":"$payload"}"""
  }

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
    val tx0Result = s"@meta/$session/0/result"
    val tx1Result = s"@meta/$session/1/result"
    val tx2Result = s"@meta/$session/2/result"
    val tx3Result = s"@meta/$session/3/result"

    "process correct tx/query sequence" in {
      // TODO: rewrite tests. 2 kinds of tests required:
      // 1. Many VM-agnostic tests for State machine logic would not depend on currently used VM impl (with VM stubbed).
      // 2. Few VM-SM tests (similar to the current one) integration tests that fix actual app_hash'es for integration.
      //
      // Currently only this test looks like integrational, other tests should be more decoupled from VM logic.

      sendCommit()
      sendCommit()
      latestAppHash shouldBe "42bb448ea02f6f4fe069f89e392315602f5463d223cbd0a8246ac42c521ea6bb"

      sendCheckTx(tx0)
      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendDeliverTx(tx0)
      sendCommit()
      latestAppHash shouldBe "7b0a908531e5936acdfce3c581ba6b39c2ca185553f47b167440490b13bfa132"

      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      latestAppHash shouldBe "fbca0d73019bc3ac6c8960782fe681835c13ace92a0d1dffd73fd363a173122c"

      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendCommit()

      sendQuery(tx1Result) shouldBe Right(Computed(littleEndian4ByteHex(2)).toStoreValue)
      sendQuery(tx3Result) shouldBe Right(Computed(littleEndian4ByteHex(4)).toStoreValue)

      latestCommittedHeight shouldBe 5
      latestAppHash shouldBe "fbca0d73019bc3ac6c8960782fe681835c13ace92a0d1dffd73fd363a173122c"
    }

    "invoke session txs in session counter order" in {
      sendCommit()
      sendCommit()

      sendDeliverTx(tx0)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      sendCommit()

      sendQuery(tx0Result) shouldBe Right(Computed(littleEndian4ByteHex(1)).toStoreValue)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendQuery(tx2Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendQuery(tx3Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))

      sendDeliverTx(tx1)
      sendCommit()
      sendCommit()

      sendQuery(tx0Result) shouldBe Right(Computed(littleEndian4ByteHex(1)).toStoreValue)
      sendQuery(tx1Result) shouldBe Right(Computed(littleEndian4ByteHex(2)).toStoreValue)
      sendQuery(tx2Result) shouldBe Right(Computed(littleEndian4ByteHex(3)).toStoreValue)
      sendQuery(tx3Result) shouldBe Right(Computed(littleEndian4ByteHex(4)).toStoreValue)
    }

    "ignore duplicated tx" in {
      sendCommit()
      sendCommit()

      sendCheckTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      sendDeliverTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      // Mempool state updated only on commit!
      sendCheckTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      sendCommit()

      sendCheckTx(tx0) shouldBe (CodeType.BAD, ClientInfoMessages.DuplicatedTransaction)
      sendDeliverTx(tx0) shouldBe (CodeType.BAD, ClientInfoMessages.DuplicatedTransaction)
    }

    "process Query method correctly" in {
      sendDeliverTx(tx0)
      sendQuery(tx0Result) shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.QueryStateIsNotReadyYet))

      sendCommit()
      sendQuery(tx0Result) shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.QueryStateIsNotReadyYet))

      sendCommit()
      sendQuery("") shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.InvalidQueryPath))
      sendQuery("/a/b/") shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.InvalidQueryPath))
      sendQuery("/a/b") shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.InvalidQueryPath))
      sendQuery("a/b/") shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.InvalidQueryPath))
      sendQuery("a//b") shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.InvalidQueryPath))
      sendQuery(tx0Result, 2) shouldBe Left((QueryCodeType.Bad, ClientInfoMessages.RequestingCustomHeightIsForbidden))
      sendQuery(tx0Result) shouldBe Right(Computed(littleEndian4ByteHex(1)).toStoreValue)
    }

    "change session summary if session explicitly closed" in {
      sendCommit()
      sendCommit()

      sendDeliverTx(tx0)
      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendDeliverTx(tx(session, 4, "@closeSession"))
      sendCommit()
      sendCommit()

      sendQuery(s"@meta/$session/4/status") shouldBe Right("sessionClosed")
      sendQuery(s"@meta/$session/@sessionSummary") shouldBe
        Right("{\"status\":{\"ExplicitlyClosed\":{}},\"invokedTxsCount\":5,\"lastTxCounter\":5}")
    }

    "expire session after expiration period elapsed" in {
      sendCommit()
      sendCommit()

      val firstSession = "000001"
      val secondSession = "000002"
      val thirdSession = "000003"
      sendDeliverTx(tx(firstSession, 0, "()"))
      sendDeliverTx(tx(secondSession, 0, "()"))
      for (i <- 0 to 5)
        sendDeliverTx(tx(thirdSession, i, "()"))
      sendDeliverTx(tx(thirdSession, 6, "@closeSession"))
      sendCommit()
      sendCommit()

      //sendQuery(s"@meta/$firstSession/@sessionSummary") shouldBe Right("{\"status\":{\"Expired\":{}},\"invokedTxsCount\":1,\"lastTxCounter\":1}")
      sendQuery(s"@meta/$secondSession/@sessionSummary") shouldBe
        Right("{\"status\":{\"Active\":{}},\"invokedTxsCount\":1,\"lastTxCounter\":2}")
      sendQuery(s"@meta/$thirdSession/@sessionSummary") shouldBe
        Right("{\"status\":{\"ExplicitlyClosed\":{}},\"invokedTxsCount\":7,\"lastTxCounter\":9}")
    }
  }
}
