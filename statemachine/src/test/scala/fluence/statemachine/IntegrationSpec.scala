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
import fluence.statemachine.tx.{Computed, Empty, Error}
import fluence.statemachine.util.{ClientInfoMessages, HexCodec}
import org.scalatest.{Matchers, OneInstancePerTest, WordSpec}

class IntegrationSpec extends WordSpec with Matchers with OneInstancePerTest {

  private val config =
    StateMachineConfig(8, List("statemachine/docker/examples/vmcode-counter"), "OFF")

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

  def tx(client: String, session: String, order: Long, payload: String, signature: String): String = {
    val txHeaderJson = s"""{"client":"$client","session":"$session","order":$order}"""
    val txJson = s"""{"header":$txHeaderJson,"payload":"$payload","timestamp":"0"}"""
    val signedTxJson = s"""{"tx":$txJson,"signature":"$signature"}"""
    HexCodec.stringToHex(signedTxJson).toUpperCase
  }

  def littleEndian4ByteHex(number: Int): String =
    Integer.toString(number, 16).reverse.padTo(8, '0').grouped(2).map(_.reverse).mkString.toUpperCase

  "State machine" should {
    val client = "client001"
    val session = "157A0E"
    val tx0 = tx(
      client,
      session,
      0,
      "inc()",
      "UY7z792BxI9os9ZjuErFyHZXONWZD1Uu1QRx9+6W40exjL+6USXFEt+3ayEEZCcu2Tv0ObJULxDhc9GQ4muuAQ"
    )
    val tx1 = tx(
      client,
      session,
      1,
      "MulModule.mul(0A0000000E000000)",
      "YsykPCUaOpHjZ45CMOUBfaQjKLvMwaeac1OZxY9BqKE658qYuMx+Loe/iZMVc6IrXFSUffEIuLWXV6weOhHwAA"
    )
    val tx2 = tx(
      client,
      session,
      2,
      "inc()",
      "T8o0+7mW1Qx/xUNUYbjRwwWL6PbxDyTFpWBIe0Ifqmyz5LLMUh4FzanHiSwBazhIcte+016TYvybgnNysa9yBA"
    )
    val tx3 = tx(
      client,
      session,
      3,
      "get()",
      "e3vs+/w5y7C4ZPqir7GlD81r+hh4QNWlAv9DymSmcVolCqpItzcY7g2G3WWtLrShn1u+yryF5kEFYx4jVeZpAw"
    )
    val tx0Failed = tx(
      client,
      session,
      0,
      "wrong()",
      "irKCx+x0aZaWlXj5+LlDyfQRwUuujC6aklY2QannQYKq5mIc3A3guSqTcTWGo3sNqeC1Wc/R+O5fClQ6uBPcAw"
    )
    val tx0Result = s"@meta/$client/$session/0/result"
    val tx1Result = s"@meta/$client/$session/1/result"
    val tx2Result = s"@meta/$client/$session/2/result"
    val tx3Result = s"@meta/$client/$session/3/result"

    "return correct initial tree root hash" in {
      sendCommit()
      // the tree containing VM state hash only
      latestAppHash shouldBe "CED687708BB077AFFF670943B503E30035E1FA4B56535B77A204BF80F5E51C3B"
    }

    "process correct tx/query sequence" in {
      sendCommit()
      sendCommit()

      sendCheckTx(tx0)
      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendDeliverTx(tx0)
      sendCommit()
      latestAppHash shouldBe "7D9A461A4E9DFE9EDC27987A312742D2A1ABE9AEB599735584FDB3EE2D1E8AB9"

      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      latestAppHash shouldBe "1C454D7C6481F54CBFC184EF5300222EBB63935566FBDACAC5C868EE3F4E9969"

      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendCommit()

      sendQuery(tx1Result) shouldBe Right(Computed(littleEndian4ByteHex(140)).toStoreValue)
      sendQuery(tx3Result) shouldBe Right(Computed(littleEndian4ByteHex(2)).toStoreValue)

      latestCommittedHeight shouldBe 5
      latestAppHash shouldBe "1C454D7C6481F54CBFC184EF5300222EBB63935566FBDACAC5C868EE3F4E9969"
    }

    "invoke session txs in session counter order" in {
      sendCommit()
      sendCommit()

      sendDeliverTx(tx0)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      sendCommit()

      sendQuery(tx0Result) shouldBe Right(Empty.toStoreValue)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendQuery(tx2Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendQuery(tx3Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))

      sendDeliverTx(tx1)
      sendCommit()
      sendCommit()

      sendQuery(tx0Result) shouldBe Right(Empty.toStoreValue)
      sendQuery(tx1Result) shouldBe Right(Computed(littleEndian4ByteHex(140)).toStoreValue)
      sendQuery(tx2Result) shouldBe Right(Empty.toStoreValue)
      sendQuery(tx3Result) shouldBe Right(Computed(littleEndian4ByteHex(2)).toStoreValue)
    }

    "ignore incorrectly signed tx" in {
      sendCommit()
      sendCommit()

      val txWithWrongSignature = tx(client, session, 0, "inc()", "bad_signature")
      sendCheckTx(txWithWrongSignature) shouldBe (CodeType.BAD, ClientInfoMessages.InvalidSignature)
      sendDeliverTx(txWithWrongSignature) shouldBe (CodeType.BAD, ClientInfoMessages.InvalidSignature)

      sendCommit()
      latestAppHash shouldBe "CED687708BB077AFFF670943B503E30035E1FA4B56535B77A204BF80F5E51C3B"
    }

    "ignore duplicated tx" in {
      sendCommit()
      sendCommit()

      sendCheckTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      sendDeliverTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      // Mempool state updated only on commit!
      sendCheckTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      sendCommit()
      latestAppHash shouldBe "7D9A461A4E9DFE9EDC27987A312742D2A1ABE9AEB599735584FDB3EE2D1E8AB9"

      sendCheckTx(tx0) shouldBe (CodeType.BAD, ClientInfoMessages.DuplicatedTransaction)
      sendDeliverTx(tx0) shouldBe (CodeType.BAD, ClientInfoMessages.DuplicatedTransaction)
      sendCommit()
      latestAppHash shouldBe "7D9A461A4E9DFE9EDC27987A312742D2A1ABE9AEB599735584FDB3EE2D1E8AB9"
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
      sendQuery(tx0Result) shouldBe Right(Empty.toStoreValue)
    }

    "change session summary if session explicitly closed" in {
      sendCommit()
      sendCommit()

      sendDeliverTx(tx0)
      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendDeliverTx(
        tx(
          client,
          session,
          4,
          "@closeSession()",
          "YocvF7duKQtr2fXWXgNQ/dG1NogRFSTYlm3h0XBZs8d39QetAt9Z/IsuIceb3BG1/ISaCSo1pdIHYRY/3xG2Ag"
        )
      )
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "33AFEDDE30BBFD2ED0398BC3A9E8B08D3C3F67486B9C4B48F4FDA227CD0BBC57"

      sendQuery(s"@meta/$client/$session/4/status") shouldBe Right("sessionClosed")
      sendQuery(s"@meta/$client/$session/@sessionSummary") shouldBe
        Right("{\"status\":{\"ExplicitlyClosed\":{}},\"invokedTxsCount\":5,\"lastTxCounter\":5}")
    }

    "not accept new txs if session failed" in {
      sendCommit()
      sendCommit()

      sendDeliverTx(tx0Failed)
      sendDeliverTx(tx1) shouldBe (CodeType.BAD, ClientInfoMessages.SessionAlreadyClosed)

      sendCommit()
      sendCommit()

      sendQuery(s"@meta/$client/$session/0/result") shouldBe
        Right(
          Error(
            "NoSuchFnError",
            "Unable to find a function with the name='<no-name>.wrong'"
          ).toStoreValue
        )
      latestAppHash shouldBe "CA1ED2F5802AB417F77E19A0C2FC4018330733A19C2BEB6CF0EFC6238A4E55EF"
    }

    "not invoke dependent txs if required failed when order in not correct" in {
      sendCommit()
      sendCommit()

      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendDeliverTx(tx0Failed)

      sendCommit()
      sendCommit()

      sendQuery(s"@meta/$client/$session/0/result") shouldBe
        Right(
          Error(
            "NoSuchFnError",
            "Unable to find a function with the name='<no-name>.wrong'"
          ).toStoreValue
        )
      sendQuery(s"@meta/$client/$session/0/status") shouldBe Right("error")
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendQuery(tx3Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      latestAppHash shouldBe "0EA9CAFF254762FF49D6AD53EE3061E0C685259E623080C8D6F36CE589170045"
    }
  }
}
