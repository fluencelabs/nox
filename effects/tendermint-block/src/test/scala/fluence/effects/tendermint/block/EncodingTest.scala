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

package fluence.effects.tendermint.block

import io.circe.parser._
import org.scalatest.{FunSpec, Matchers, OptionValues}
import scalapb.GeneratedMessage
import scodec.bits.ByteVector

class EncodingTest extends FunSpec with Matchers with OptionValues {

  val block = JSON.block(TestData.blockResponse).right.get
  val vote = JSON.vote(parse(TestData.vote).right.get)

  def toHex(ba: Array[Byte]) = ByteVector(ba).toHex
  def checkHex(scalaHex: String, goHex: String) = scalaHex.toLowerCase shouldBe goHex.toLowerCase

  it("encode vote") {
    val voteHex = toHex(Amino.encode(vote))
    val goHex =
      "0802101022480a201e56cf404964aa6b0768e67ad9cbacabcebcd6a84dc0fc924f1c0af9043c0188122408011220d0a00d1902638e1f4fd625568d4a4a7d9fc49e8f3586f257535fc835e7b0b7852a0c08dbd4dce50510f7e6e0ff01321404c60b72246943675e2f3aada00e30ec41aa7d4e4240674f7172b7f3f53ebedead5893e831528dada1d3c8ee679b29e77accefa2d699c830f7456d2153f49263c68e49f472eb9f8c7616a7f741879f43c95018d33306"
    checkHex(voteHex, goHex)
  }

  it("encode blockID") {
    block.header.last_block_id shouldBe defined
    val blockId = block.header.last_block_id.value
    val scalaHex = toHex(Amino.encode(blockId))
    val goHex =
      "0A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"

    checkHex(scalaHex, goHex)
  }

  it("encode Version") {
    block.header.version shouldBe defined
    val version = block.header.version.value
    val scalaHex = toHex(Amino.encode(version))
    val goHex = "080A"

    checkHex(scalaHex, goHex)
  }

  it("encode string") {
    val s = "aaa"
    val scalaHex = toHex(Amino.encode(s))
    val goHex = "03616161"

    checkHex(scalaHex, goHex)
  }

  it("encode uint64") {
    val l = 123456789L
    val scalaHex = toHex(Amino.encode(l))
    val goHex = "959AEF3A"

    checkHex(scalaHex, goHex)
  }

  it("encode bytes") {
    val arr = ByteVector(1, 2, 3, 4, 6, 7, 8, 9, 10)
    val scalaHex = toHex(Amino.encode(arr))
    val goHex = "0901020304060708090A"

    checkHex(scalaHex, goHex)
  }
}
