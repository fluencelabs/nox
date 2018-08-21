/*
 * Copyright (C) 2017  Fluence Labs Limited
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

package fluence.statemachine

import com.github.jtendermint.jabci.api.CodeType
import com.github.jtendermint.jabci.types.{RequestCheckTx, RequestCommit, RequestDeliverTx, RequestQuery}
import com.google.protobuf.ByteString
import fluence.statemachine.util.HexCodec
import org.scalatest.{Matchers, OneInstancePerTest, WordSpec}

class IntegrationSpec extends WordSpec with Matchers with OneInstancePerTest {
  val abciHandler: ABCIHandler = new ABCIHandler

  def sendCheckTx(tx: String): Int =
    abciHandler.requestCheckTx(RequestCheckTx.newBuilder().setTx(ByteString.copyFromUtf8(tx)).build()).getCode

  def sendDeliverTx(tx: String): Int =
    abciHandler.receivedDeliverTx(RequestDeliverTx.newBuilder().setTx(ByteString.copyFromUtf8(tx)).build()).getCode

  def sendCommit(): Unit =
    abciHandler.requestCommit(RequestCommit.newBuilder().build())

  def sendQuery(query: String, height: Int = 0): Either[String, String] = {
    val builtQuery = RequestQuery.newBuilder().setHeight(height).setPath(query).setProve(true).build()
    val response = abciHandler.requestQuery(builtQuery)
    response.getCode match {
      case CodeType.OK => Right(response.getValue.toStringUtf8)
      case _ => Left(response.getInfo)
    }
  }

  def latestAppHash(): String = abciHandler.stateHolder.mempoolState.merkleHash.toHex

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
      "increment(counter1)",
      "P10vIMUJ14vgPaH4hzJpUgdWaTl2PWXHN55HhtmCogP5OaDmY9uMzdts9nYcwhsFjKd80+EV/XLFs6bDmZFuAQ"
    )
    val tx1 = tx(
      client,
      session,
      1,
      "increment(counter1)",
      "wtFgfLfG6XeVs/EZtvCmPwZcaomaii3q/L/B7eD0Whnx34lO1mBDvWEWW4E1oYuY07K9sI9ZRcdO//H1z79KCg"
    )
    val tx2 = tx(
      client,
      session,
      2,
      "increment(counter1)",
      "L78qWafcGsbasUanscBStlDsuiegU163ospoTwBzDjgzgq9Hu+5kgudaaCscSJn48uLlywm5jeN8bLTVUHC7Bg"
    )
    val tx3 = tx(
      client,
      session,
      3,
      "sum(12,34)",
      "488/Y4cXgqc1PNZrCKWKlidcQJp345UtXAtW+qNnF11ZIq5m1NYm2oqHHRfVmgDt4aqlvwNp0cMV4w1diSgrBA"
    )
    val tx0Result = s"@meta/$client/$session/0/result"
    val tx1Result = s"@meta/$client/$session/1/result"
    val tx2Result = s"@meta/$client/$session/2/result"
    val tx3Result = s"@meta/$client/$session/3/result"

    "return correct empty tree root hash" in {
      sendCommit()
      // corresponds to SHA-3-256 value of empty string from table here: https://www.di-mgt.com.au/sha_testvectors.html
      latestAppHash() shouldBe "A7FFC6F8BF1ED76651C14756A061D662F580FF4DE43B49FA82D80A4B80F8434A"
    }

    "process correct tx/query sequence" in {
      sendCommit()
      sendCommit()
      latestAppHash() shouldBe "A7FFC6F8BF1ED76651C14756A061D662F580FF4DE43B49FA82D80A4B80F8434A"

      sendCheckTx(tx0)
      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Right("")
      sendDeliverTx(tx0)
      sendCommit()
      latestAppHash() shouldBe "EB151EAC18A2B7363BDE730A0927743E676A10B88DBB5F268CF8E1DDA0237C1D"

      sendCheckTx(tx1)
      sendCheckTx(tx2)
      sendCheckTx(tx3)
      sendQuery(tx1Result) shouldBe Right("")
      sendDeliverTx(tx1)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      latestAppHash() shouldBe "EC6692C8DDF27AAB7960AB36F5C3C57DFFA54D5A723E75590A1B9859E7BF9DB5"

      sendQuery(tx1Result) shouldBe Right("")
      sendCommit()

      sendQuery(tx1Result) shouldBe Right("2")
      sendQuery(tx3Result) shouldBe Right("46")

      abciHandler.stateHolder.latestCommittedHeight shouldBe 5
      latestAppHash() shouldBe "EC6692C8DDF27AAB7960AB36F5C3C57DFFA54D5A723E75590A1B9859E7BF9DB5"
    }

    "invoke session txs in session counter order" in {
      sendCommit()
      sendCommit()

      sendDeliverTx(tx0)
      sendDeliverTx(tx2)
      sendDeliverTx(tx3)
      sendCommit()
      sendCommit()

      sendQuery(tx0Result) shouldBe Right("1")
      sendQuery(tx1Result) shouldBe Right("")
      sendQuery(tx2Result) shouldBe Right("")
      sendQuery(tx3Result) shouldBe Right("")

      sendDeliverTx(tx1)
      sendCommit()
      sendCommit()

      sendQuery(tx1Result) shouldBe Right("2")
      sendQuery(tx2Result) shouldBe Right("3")
      sendQuery(tx3Result) shouldBe Right("46")
    }

    "ignore duplicated tx" in {
      sendCommit()
      sendCommit()
      latestAppHash() shouldBe "A7FFC6F8BF1ED76651C14756A061D662F580FF4DE43B49FA82D80A4B80F8434A"

      sendCheckTx(tx0) shouldBe CodeType.OK
      sendDeliverTx(tx0) shouldBe CodeType.OK
      sendCheckTx(tx0) shouldBe CodeType.OK // Mempool state updated only on commit!
      sendCommit()
      latestAppHash() shouldBe "EB151EAC18A2B7363BDE730A0927743E676A10B88DBB5F268CF8E1DDA0237C1D"

      sendCheckTx(tx0) shouldBe CodeType.BAD
      sendDeliverTx(tx0) shouldBe CodeType.BAD
      sendCommit()
      latestAppHash() shouldBe "EB151EAC18A2B7363BDE730A0927743E676A10B88DBB5F268CF8E1DDA0237C1D"
    }

    "process Query method correctly" in {
      sendQuery(tx0Result) shouldBe Left("Query state is not ready yet")

      sendCommit()
      sendQuery(tx0Result) shouldBe Left("Query state is not ready yet")

      sendCommit()
      sendQuery("") shouldBe Left("Invalid query path")
      sendQuery("/a/b/") shouldBe Left("Invalid query path")
      sendQuery("/a/b") shouldBe Left("Invalid query path")
      sendQuery("a/b/") shouldBe Left("Invalid query path")
      sendQuery("a//b") shouldBe Left("Invalid query path")
      sendQuery(tx0Result, 2) shouldBe Left("Requesting custom height is forbidden")
      sendQuery(tx0Result) shouldBe Right("")
    }
  }
}
