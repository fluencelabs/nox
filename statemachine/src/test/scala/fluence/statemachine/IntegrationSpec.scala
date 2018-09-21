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

  val abciHandler: AbciHandler =
    ServerRunner.buildAbciHandler().valueOr(e => throw new RuntimeException(e.message)).unsafeRunSync()

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
      latestAppHash shouldBe "74712CAAABC7F02D055461B5A6CE5B573E65ADA57809869EFE589DF07FBAF0CA"
    }

    "process correct tx/query sequence" in {
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "74712CAAABC7F02D055461B5A6CE5B573E65ADA57809869EFE589DF07FBAF0CA"

      sendCheckTx(tx0)
      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendDeliverTx(tx0)
      sendCommit()
      latestAppHash shouldBe "9CF0B1A8ECC2DC463CA5029E72F08EBC5BB3560B592BBD645D7A29699466511E"

      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      latestAppHash shouldBe "E7CA2973D0E0CF4ECBED00A3B107D3174FB33E8F3B458A5940E643D268104CB3"

      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendCommit()

      sendQuery(tx1Result) shouldBe Right(Computed("140").toStoreValue)
      sendQuery(tx3Result) shouldBe Right(Computed("2").toStoreValue)

      latestCommittedHeight shouldBe 5
      latestAppHash shouldBe "E7CA2973D0E0CF4ECBED00A3B107D3174FB33E8F3B458A5940E643D268104CB3"
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
      latestAppHash shouldBe "74712CAAABC7F02D055461B5A6CE5B573E65ADA57809869EFE589DF07FBAF0CA"

      val txWithWrongSignature = tx(client, session, 0, "inc()", "bad_signature")
      sendCheckTx(txWithWrongSignature) shouldBe (CodeType.BAD, ClientInfoMessages.InvalidSignature)
      sendDeliverTx(txWithWrongSignature) shouldBe (CodeType.BAD, ClientInfoMessages.InvalidSignature)

      sendCommit()
      latestAppHash shouldBe "74712CAAABC7F02D055461B5A6CE5B573E65ADA57809869EFE589DF07FBAF0CA"
    }

    "ignore duplicated tx" in {
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "74712CAAABC7F02D055461B5A6CE5B573E65ADA57809869EFE589DF07FBAF0CA"

      sendCheckTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      sendDeliverTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      // Mempool state updated only on commit!
      sendCheckTx(tx0) shouldBe (CodeType.OK, ClientInfoMessages.SuccessfulTxResponse)
      sendCommit()
      latestAppHash shouldBe "9CF0B1A8ECC2DC463CA5029E72F08EBC5BB3560B592BBD645D7A29699466511E"

      sendCheckTx(tx0) shouldBe (CodeType.BAD, ClientInfoMessages.DuplicatedTransaction)
      sendDeliverTx(tx0) shouldBe (CodeType.BAD, ClientInfoMessages.DuplicatedTransaction)
      sendCommit()
      latestAppHash shouldBe "9CF0B1A8ECC2DC463CA5029E72F08EBC5BB3560B592BBD645D7A29699466511E"
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
      latestAppHash shouldBe "74712CAAABC7F02D055461B5A6CE5B573E65ADA57809869EFE589DF07FBAF0CA"

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
          "fADQUq3sxia+WRo9vUb0W+ZlBISlnwlCT5zhfSNBw3/KbIOUkNCRAJx2q0pSMH8b537jDCCZ1ZkIw8IHr3g/CA"
        )
      )
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "9728D3AA909D48ACC56EDB3EC93BDA73A245FB4BE0568D508A08B76EE3344DDA"

      sendQuery(s"@meta/$client/$session/4/status") shouldBe Right("sessionClosed")
      sendQuery(s"@meta/$client/$session/@sessionSummary") shouldBe
        Right("{\"status\":{\"ExplicitlyClosed\":{}},\"invokedTxsCount\":5,\"lastTxCounter\":5}")
    }

    "not accept new txs if session failed" in {
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "74712CAAABC7F02D055461B5A6CE5B573E65ADA57809869EFE589DF07FBAF0CA"

      sendDeliverTx(tx0Failed)
      sendCommit()
      latestAppHash shouldBe "F5E48F5F4F45AF6CEE6EF45623C2FFFD991E4D40B42C871ABC1AB5746F639DAD"

      sendDeliverTx(tx1) shouldBe (CodeType.BAD, ClientInfoMessages.SessionAlreadyClosed)
      sendCommit()
      latestAppHash shouldBe "F5E48F5F4F45AF6CEE6EF45623C2FFFD991E4D40B42C871ABC1AB5746F639DAD"
    }

    "not invoke dependent txs if required failed when order in not correct" in {
      sendCommit()
      sendCommit()
      latestAppHash shouldBe "74712CAAABC7F02D055461B5A6CE5B573E65ADA57809869EFE589DF07FBAF0CA"

      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendDeliverTx(tx0Failed)
      sendCommit()
      latestAppHash shouldBe "2213B30488CAA0DF787B156DA6345400204960E4033776C9BB08FA9AB055800F"

      sendCommit()
      sendQuery(tx1Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      sendQuery(tx3Result) shouldBe Left((QueryCodeType.NotReady, ClientInfoMessages.ResultIsNotReadyYet))
      latestAppHash shouldBe "2213B30488CAA0DF787B156DA6345400204960E4033776C9BB08FA9AB055800F"
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
