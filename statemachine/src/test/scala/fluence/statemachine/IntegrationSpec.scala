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

import com.github.jtendermint.jabci.api.CodeType
import com.github.jtendermint.jabci.types.{RequestCheckTx, RequestCommit, RequestDeliverTx, RequestQuery}
import com.google.protobuf.ByteString
import fluence.statemachine.config.StateMachineConfig
import fluence.statemachine.state.QueryCodeType
import fluence.statemachine.tree.MerkleTreeNode
import fluence.statemachine.tx.{Computed, Error}
import fluence.statemachine.util.{ClientInfoMessages, HexCodec}
import org.scalatest.{Matchers, OneInstancePerTest, WordSpec}

class IntegrationSpec extends WordSpec with Matchers with OneInstancePerTest {

  // sbt defaults user directory to submodule directory
  // while Idea defaults to project root
  private val moduleDirPrefix = if (System.getProperty("user.dir").endsWith("/statemachine")) "../" else "./"
  private val moduleFiles = List("mul.wast", "counter.wast").map(moduleDirPrefix + "vm/src/test/resources/wast/" + _)
  private val config = StateMachineConfig(8, moduleFiles, "OFF")

  val abciHandler: AbciHandler = ServerRunner
    .buildAbciHandler(config)
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

  def latestAppHash: String = latestCommittedState.merkleHash.toHex

  def tx(client: SigningClient, session: String, order: Long, payload: String, signature: String): String = {
    val txHeaderJson = s"""{"client":"${client.id}","session":"$session","order":$order}"""
    val txJson = s"""{"header":$txHeaderJson,"payload":"$payload","timestamp":"0"}"""
    val signedTxJson = s"""{"tx":$txJson,"signature":"$signature"}"""
    HexCodec.stringToHex(signedTxJson).toUpperCase
  }

  def tx(client: SigningClient, session: String, order: Long, payload: String): String = {
    val txHeaderJson = s"""{"client":"${client.id}","session":"$session","order":$order}"""
    val txJson = s"""{"header":$txHeaderJson,"payload":"$payload"}"""
    val signingData = s"${client.id}-$session-$order-$payload"
    val signedTxJson = s"""{"tx":$txJson,"signature":"${client.sign(signingData)}"}"""
    HexCodec.stringToHex(signedTxJson).toUpperCase
  }

  def littleEndian4ByteHex(number: Int): String =
    Integer.toString(number, 16).reverse.padTo(8, '0').grouped(2).map(_.reverse).mkString.toUpperCase

  "State machine" should {
    val client = SigningClient(
      "client001",
      "TVAD4tNeMH2yJfkDZBSjrMJRbavmdc3/fGU2N2VAnxQ"
    )
    val session = "157A0E"
    val tx0 = tx(
      client,
      session,
      0,
      "()",
      "L20to3vLwexFgUC1XgOaCKKNCxo433ScYc+EKBQdnMpIqlUOifG4Vn/9qL1OhLpBUGKOYRFBi3l517Uu37mOAQ=="
    )
    val tx1 = tx(
      client,
      session,
      1,
      "()",
      "GnGwQ/sKW2m8HqvigBRlmljOJhkAGbnslyQ4UYkWtnnvyzvveX9YTQUCZ4cFpL5ZsugaVHMqGBFn5ERN5UWzBA=="
    )
    val tx2 = tx(client, session, 2, "()")
    val tx3 = tx(client, session, 3, "()")
    val tx0Result = s"@meta/${client.id}/$session/0/result"
    val tx1Result = s"@meta/${client.id}/$session/1/result"
    val tx2Result = s"@meta/${client.id}/$session/2/result"
    val tx3Result = s"@meta/${client.id}/$session/3/result"

    "process correct tx/query sequence" in {
      // TODO: rewrite tests. 2 kinds of tests required:
      // 1. Many VM-agnostic tests for State machine logic would not depend on currently used VM impl (with VM stubbed).
      // 2. Few VM-SM tests (similar to the current one) integration tests that fix actual app_hash'es for integration.
      //
      // Currently only this test looks like integrational, other tests should be more decoupled from VM logic.

      sendCommit()
      sendCommit()
      latestAppHash shouldBe "CED687708BB077AFFF670943B503E30035E1FA4B56535B77A204BF80F5E51C3B"

      sendCheckTx(tx0)
      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendDeliverTx(tx0)
      sendCommit()
      latestAppHash shouldBe "501B4A2372F91385D08BF276F16167D1A5BAF074C91B630C5E582E15D6F3F23E"

      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      latestAppHash shouldBe "D1FACD887C07449BC198538645D3AE86107746B9F887C402876FC50C26C2F3BD"

      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendCommit()

      sendQuery(tx1Result) shouldBe Right(Computed(littleEndian4ByteHex(2)).toStoreValue)
      sendQuery(tx3Result) shouldBe Right(Computed(littleEndian4ByteHex(4)).toStoreValue)

      latestCommittedHeight shouldBe 5
      latestAppHash shouldBe "D1FACD887C07449BC198538645D3AE86107746B9F887C402876FC50C26C2F3BD"
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

    "ignore incorrectly signed tx" in {
      sendCommit()
      sendCommit()

      val txWithWrongSignature = tx(client, session, 0, "()", "bad_signature")
      sendCheckTx(txWithWrongSignature) shouldBe (CodeType.BAD, ClientInfoMessages.InvalidSignature)
      sendDeliverTx(txWithWrongSignature) shouldBe (CodeType.BAD, ClientInfoMessages.InvalidSignature)
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
      sendDeliverTx(tx(client, session, 4, "@closeSession"))
      sendCommit()
      sendCommit()

      sendQuery(s"@meta/${client.id}/$session/4/status") shouldBe Right("sessionClosed")
      sendQuery(s"@meta/${client.id}/$session/@sessionSummary") shouldBe
        Right("{\"status\":{\"ExplicitlyClosed\":{}},\"invokedTxsCount\":5,\"lastTxCounter\":5}")
    }

    "expire session after expiration period elapsed" in {
      sendCommit()
      sendCommit()

      val firstSession = "000001"
      val secondSession = "000002"
      val thirdSession = "000003"
      sendDeliverTx(tx(client, firstSession, 0, "()"))
      sendDeliverTx(tx(client, secondSession, 0, "()"))
      for (i <- 0 to 5)
        sendDeliverTx(tx(client, thirdSession, i, "()"))
      sendDeliverTx(tx(client, thirdSession, 6, "@closeSession"))
      sendCommit()
      sendCommit()

      sendQuery(s"@meta/${client.id}/$firstSession/@sessionSummary") shouldBe
        Right("{\"status\":{\"Expired\":{}},\"invokedTxsCount\":1,\"lastTxCounter\":1}")
      sendQuery(s"@meta/${client.id}/$secondSession/@sessionSummary") shouldBe
        Right("{\"status\":{\"Active\":{}},\"invokedTxsCount\":1,\"lastTxCounter\":2}")
      sendQuery(s"@meta/${client.id}/$thirdSession/@sessionSummary") shouldBe
        Right("{\"status\":{\"ExplicitlyClosed\":{}},\"invokedTxsCount\":7,\"lastTxCounter\":9}")
    }
  }
}
