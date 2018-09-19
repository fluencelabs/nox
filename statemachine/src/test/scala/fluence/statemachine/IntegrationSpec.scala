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
import fluence.statemachine.state.QueryCodeType
import fluence.statemachine.tree.MerkleTreeNode
import fluence.statemachine.tx.{Computed, Empty, Error}
import fluence.statemachine.util.{ClientInfoMessages, HexCodec}
import org.scalatest.{Matchers, OneInstancePerTest, WordSpec}

class IntegrationSpec extends WordSpec with Matchers with OneInstancePerTest {
  val abciHandler: AbciHandler = ServerRunner.buildAbciHandler().unsafeRunSync()

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
    val txJson = s"""{"header":$txHeaderJson,"payload":"$payload"}"""
    val signedTxJson = s"""{"tx":$txJson,"signature":"$signature"}"""
    HexCodec.stringToHex(signedTxJson).toUpperCase
  }

  "State machine" should {
    val client = "client001"
    val session = "157A0E"
    val tx0 = tx(
      client,
      session,
      0,
      "inc()",
      "j8jWIEomQme35ozNzzk6PZVsArYSYWsVvmhbPKFirJv44uDTXapZuAk2vu5hSQ6O1zup7RzmOdKZiGFPOVlSDQ"
    )
    val tx1 = tx(
      client,
      session,
      1,
      "multiplier.mul(10,14)",
      "CeXRfIfJ/jf6zFbsO+Fku6l8tpySM4RggyMoIcGGSC84FtiXY/aHXRJbYDwrMdxspWWgDwqbyiwAF5DUk7GxBg"
    )
    val tx2 = tx(
      client,
      session,
      2,
      "inc()",
      "BfZXvxjC4YsDEMzneumKZwZWX84Xd1MOl7qhlFyhkMmKlxqs8U9IUpz3mr6aIzsW6u+PdeGmxsOJ18uDyKVlAA"
    )
    val tx3 = tx(
      client,
      session,
      3,
      "get()",
      "19y3VBoB3ccmiuvO1d8ZoR4Du12Oz5pZ9L2tNxUkCCmWAL+/1sX+1Iy3OyH6kT2E6PSJw6j5f0Fpfyu91fj5Aw"
    )
    val tx0Failed = tx(
      client,
      session,
      0,
      "multiplier.mul(12,a)",
      "mIha9cksj1brqH86VwXiN/rj+B44DBgqC7Kee2w6MOzWvlAZsJkWOQvhzip2pV6eHNWQqQXCGhIEasTSf4iCDg"
    )
    val tx0Result = s"@meta/$client/$session/0/result"
    val tx1Result = s"@meta/$client/$session/1/result"
    val tx2Result = s"@meta/$client/$session/2/result"
    val tx3Result = s"@meta/$client/$session/3/result"

    "return correct initial tree root hash" in {
      sendCommit()
      // the tree containing VM state hash only
      latestAppHash shouldBe "8C1BD484965551A9E4E43951A82366CF75977D19A967E516F048BE4C32F6122A"
    }

    "process correct tx/query sequence" in {
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "8C1BD484965551A9E4E43951A82366CF75977D19A967E516F048BE4C32F6122A"

      sendCheckTx(tx0)
      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendDeliverTx(tx0)
      sendCommit()
      latestAppHash shouldBe "49C50F758E31D60AB958DBF08B9EC93413B671F39F1007CE99279883051DA6A8"

      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      latestAppHash shouldBe "69D790266D8047DEBE2AE7DC5B1A7424EE6AA4319E8A09D98AB39556C8D0D61F"

      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendCommit()

      sendQuery(tx1Result) shouldBe Right(Computed("140").toStoreValue)
      sendQuery(tx3Result) shouldBe Right(Computed("2").toStoreValue)

      latestCommittedHeight shouldBe 5
      latestAppHash shouldBe "69D790266D8047DEBE2AE7DC5B1A7424EE6AA4319E8A09D98AB39556C8D0D61F"
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
      sendQuery(tx1Result) shouldBe Right(Computed("140").toStoreValue)
      sendQuery(tx2Result) shouldBe Right(Empty.toStoreValue)
      sendQuery(tx3Result) shouldBe Right(Computed("2").toStoreValue)
    }

    "ignore incorrectly signed tx" in {
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "8C1BD484965551A9E4E43951A82366CF75977D19A967E516F048BE4C32F6122A"

      val txWithWrongSignature = tx(client, session, 0, "inc()", "bad_signature")
      sendCheckTx(txWithWrongSignature) shouldBe (CodeType.BAD, ClientInfoMessages.InvalidSignature)
      sendDeliverTx(txWithWrongSignature) shouldBe (CodeType.BAD, ClientInfoMessages.InvalidSignature)

      sendCommit()
      latestAppHash shouldBe "8C1BD484965551A9E4E43951A82366CF75977D19A967E516F048BE4C32F6122A"
    }

    "ignore duplicated tx" in {
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "8C1BD484965551A9E4E43951A82366CF75977D19A967E516F048BE4C32F6122A"

      sendCheckTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      sendDeliverTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      // Mempool state updated only on commit!
      sendCheckTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      sendCommit()
      latestAppHash shouldBe "49C50F758E31D60AB958DBF08B9EC93413B671F39F1007CE99279883051DA6A8"

      sendCheckTx(tx0) shouldBe (CodeType.BAD, ClientInfoMessages.DuplicatedTransaction)
      sendDeliverTx(tx0) shouldBe (CodeType.BAD, ClientInfoMessages.DuplicatedTransaction)
      sendCommit()
      latestAppHash shouldBe "49C50F758E31D60AB958DBF08B9EC93413B671F39F1007CE99279883051DA6A8"
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

    "not invoke dependent txs if required failed when order is correct" in {
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "8C1BD484965551A9E4E43951A82366CF75977D19A967E516F048BE4C32F6122A"

      sendDeliverTx(tx0Failed)
      sendCommit()
      latestAppHash shouldBe "C572702229EAF213E32F37A0F7EB5F08FB152D2E7E248CBEAD36759D0CEE3D56"

      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      latestAppHash shouldBe "72E1053895C59C9A8106A0511B8D2B9C9B96FFB0E6496DAAD89002EAB6E544CC"

      sendCommit()
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendQuery(tx3Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      latestCommittedHeight shouldBe 5
      latestAppHash shouldBe "72E1053895C59C9A8106A0511B8D2B9C9B96FFB0E6496DAAD89002EAB6E544CC"
    }

    "not invoke dependent txs if required failed when order in not correct" in {
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "8C1BD484965551A9E4E43951A82366CF75977D19A967E516F048BE4C32F6122A"

      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendDeliverTx(tx0Failed)
      sendCommit()
      latestAppHash shouldBe "72E1053895C59C9A8106A0511B8D2B9C9B96FFB0E6496DAAD89002EAB6E544CC"
    }

    "store error message for incorrect operations" in {
      sendCommit()
      sendCommit()

      val session1 = "000001"
      val wrongArgTx1 = tx(
        client,
        session1,
        0,
        "inc(5)",
        "FJzdcaHohmrz1ZMxMnrJbmiLzxclWoaMx9j87VYSR7BruAhfRHV0Q1MPJIv5wcbxeaeYgpLyGvzfdecQcWWnDw"
      )
      sendDeliverTx(wrongArgTx1)
      sendCommit()
      sendCommit()

      val expectedNumOfArgsMessage = "Invalid number of arguments, expected=0, actually=1 for fn='<no-name>.inc'"
      sendQuery(s"@meta/$client/$session1/0/result") shouldBe
        Right(Error("InvalidArgError$", expectedNumOfArgsMessage).toStoreValue)
      sendQuery(s"@meta/$client/$session1/0/status") shouldBe Right("error")

      val session2 = "000002"
      val wrongArgTx2 = tx(
        client,
        session2,
        0,
        "unknown()",
        "jtXG+Iq0jxqrHAJzLQHrQoBdkoZj3QdfUdsCd29flYT9V9vfCh6T+LmN+2rtsH6ERbfoXVVq6Tw4amXPYaSJDw"
      )
      sendDeliverTx(wrongArgTx2)
      sendCommit()
      sendCommit()

      sendQuery(s"@meta/$client/$session2/0/result") shouldBe
        Right(Error("NoSuchFnError$", "Unable to find a function with the name='<no-name>.unknown'").toStoreValue)
      sendQuery(s"@meta/$client/$session2/0/status") shouldBe Right("error")
    }
  }
}
