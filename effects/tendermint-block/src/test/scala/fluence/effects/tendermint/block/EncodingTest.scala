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
import scodec.bits.ByteVector

class EncodingTest extends FunSpec with Matchers with OptionValues {

  val block = JSON.block(TestData.blockResponse).right.get.fillHeader()
  val vote = JSON.vote(parse(TestData.vote).right.get)

  def toHex(ba: Array[Byte]) = ByteVector(ba).toHex
  def checkHex(scalaHex: String, goHex: String) = scalaHex.toLowerCase shouldBe goHex.toLowerCase

  describe("header") {
    it("version") { checkHex(toHex(Amino.encode(block.header.version)), "080A") }

    it("chain_id") { checkHex(toHex(Amino.encode(block.header.chain_id)), "023130") }

    it("height") { checkHex(toHex(Amino.encode(block.header.height)), "11") }

    it("time") { checkHex(toHex(Amino.encode(block.header.time)), "08DBD4DCE50510F7E6E0FF01") }

    it("num_txs") { checkHex(toHex(Amino.encode(block.header.num_txs)), "04") }

    it("total_txs") { checkHex(toHex(Amino.encode(block.header.total_txs)), "3A") }

    it("last_block_id") {
      checkHex(
        toHex(Amino.encode(block.header.last_block_id)),
        "0A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      )
    }

    it("last_commit_hash") {
      checkHex(
        toHex(Amino.encode(block.header.last_commit_hash)),
        "204B2FE08AE3C3F7573DB0A36250FB2AABCB1893638EBBECAB7356DDCEF358B423"
      )
    }

    it("data_hash") {
      checkHex(
        toHex(Amino.encode(block.header.data_hash)),
        "2061135E296D5570C607C456416CBD6827E21DED57F7FE2F08879ADCC8228FA7C0"
      )
    }

    it("validators_hash") {
      checkHex(
        toHex(Amino.encode(block.header.validators_hash)),
        "20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374"
      )
    }

    it("next_validators_hash") {
      checkHex(
        toHex(Amino.encode(block.header.next_validators_hash)),
        "20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374"
      )
    }

    it("consensus_hash") {
      checkHex(
        toHex(Amino.encode(block.header.consensus_hash)),
        "20048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F"
      )
    }

    it("app_hash") {
      checkHex(
        toHex(Amino.encode(block.header.app_hash)),
        "20DA265DD0F482C2D76B7E67B0128B391FED2971CD74E6DED3EDCCBAB200653DAE"
      )
    }

    it("last_results_hash") {
      checkHex(
        toHex(Amino.encode(block.header.last_results_hash)),
        "207FD69E789D2A445E0F3258965481A2E70D5EB9960BCAF065B08C417EF045A846"
      )
    }

    it("evidence_hash") {
      checkHex(toHex(Amino.encode(block.header.evidence_hash)), "")
    }

    it("proposer_address") {
      checkHex(toHex(Amino.encode(block.header.proposer_address)), "1404C60B72246943675E2F3AADA00E30EC41AA7D4E")
    }
  }

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
