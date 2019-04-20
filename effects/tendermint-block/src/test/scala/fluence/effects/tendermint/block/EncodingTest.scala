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
        toHex(Amino.encode(block.header.last_commit_hash, skipEmpty = true)),
        "204B2FE08AE3C3F7573DB0A36250FB2AABCB1893638EBBECAB7356DDCEF358B423"
      )
    }

    it("data_hash") {
      checkHex(
        toHex(Amino.encode(block.header.data_hash, skipEmpty = true)),
        "2061135E296D5570C607C456416CBD6827E21DED57F7FE2F08879ADCC8228FA7C0"
      )
    }

    it("validators_hash") {
      checkHex(
        toHex(Amino.encode(block.header.validators_hash, skipEmpty = true)),
        "20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374"
      )
    }

    it("next_validators_hash") {
      checkHex(
        toHex(Amino.encode(block.header.next_validators_hash, skipEmpty = true)),
        "20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374"
      )
    }

    it("consensus_hash") {
      checkHex(
        toHex(Amino.encode(block.header.consensus_hash, skipEmpty = true)),
        "20048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F"
      )
    }

    it("app_hash") {
      checkHex(
        toHex(Amino.encode(block.header.app_hash, skipEmpty = true)),
        "20DA265DD0F482C2D76B7E67B0128B391FED2971CD74E6DED3EDCCBAB200653DAE"
      )
    }

    it("last_results_hash") {
      checkHex(
        toHex(Amino.encode(block.header.last_results_hash, skipEmpty = true)),
        "207FD69E789D2A445E0F3258965481A2E70D5EB9960BCAF065B08C417EF045A846"
      )
    }

    it("evidence_hash") {
      checkHex(toHex(Amino.encode(block.header.evidence_hash, skipEmpty = true)), "")
    }

    it("proposer_address") {
      checkHex(
        toHex(Amino.encode(block.header.proposer_address, skipEmpty = true)),
        "1404C60B72246943675E2F3AADA00E30EC41AA7D4E"
      )
    }
  }

  describe("amino header, length prefixed") {
    val aminoHeader = AminoBlock.toAminoHeader(block.header)

    it("Version") {
      val simpleBytes = Amino.encode(aminoHeader.version)
      val goSize = 2
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "080A"
      checkHex(scalaHex, goHex)
    }

    it("ChainID") {
      val simpleBytes = Amino.encode(aminoHeader.chainId)
      val goSize = 3
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "023130"
      checkHex(scalaHex, goHex)
    }

    it("Height") {
      val simpleBytes = Amino.encode(aminoHeader.height)
      val goSize = 1
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "11"
      checkHex(scalaHex, goHex)
    }

    it("Time") {
      val simpleBytes = Amino.encode(aminoHeader.time)
      val goSize = 12
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "08DBD4DCE50510F7E6E0FF01"
      checkHex(scalaHex, goHex)
    }

    it("NumTxs") {
      val simpleBytes = Amino.encode(aminoHeader.numTxs)
      val goSize = 1
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "04"
      checkHex(scalaHex, goHex)
    }

    it("TotalTxs") {
      val simpleBytes = Amino.encode(aminoHeader.totalTxs)
      val goSize = 1
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "3A"
      checkHex(scalaHex, goHex)
    }

    it("LastBlockID") {
      val simpleBytes = Amino.encode(aminoHeader.lastBlockId)
      val goSize = 72
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex =
        "0A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      checkHex(scalaHex, goHex)
    }

    it("LastCommitHash") {
      val simpleBytes = Amino.encode(aminoHeader.lastCommitHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "204B2FE08AE3C3F7573DB0A36250FB2AABCB1893638EBBECAB7356DDCEF358B423"
      checkHex(scalaHex, goHex)
    }

    it("DataHash") {
      val simpleBytes = Amino.encode(aminoHeader.dataHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "2061135E296D5570C607C456416CBD6827E21DED57F7FE2F08879ADCC8228FA7C0"
      checkHex(scalaHex, goHex)
    }

    it("ValidatorsHash") {
      val simpleBytes = Amino.encode(aminoHeader.validatorsHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374"
      checkHex(scalaHex, goHex)
    }

    it("NextValidatorsHash") {
      val simpleBytes = Amino.encode(aminoHeader.nextValidatorsHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374"
      checkHex(scalaHex, goHex)
    }

    it("ConsensusHash") {
      val simpleBytes = Amino.encode(aminoHeader.consensusHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "20048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F"
      checkHex(scalaHex, goHex)
    }

    it("AppHash") {
      val simpleBytes = Amino.encode(aminoHeader.appHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "20DA265DD0F482C2D76B7E67B0128B391FED2971CD74E6DED3EDCCBAB200653DAE"
      checkHex(scalaHex, goHex)
    }

    it("LastResultsHash") {
      val simpleBytes = Amino.encode(aminoHeader.lastResultsHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "207FD69E789D2A445E0F3258965481A2E70D5EB9960BCAF065B08C417EF045A846"
      checkHex(scalaHex, goHex)
    }

    it("EvidenceHash, skipEmpty = true") {
      val simpleBytes = Amino.encode(aminoHeader.evidenceHash, skipEmpty = true)
      val goSize = 0
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = ""
      checkHex(scalaHex, goHex)
    }

    it("EvidenceHash, skipEmpty = false") {
      val simpleBytes = Amino.encode(aminoHeader.evidenceHash, skipEmpty = false)
      val goSize = 1
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "00"
      checkHex(scalaHex, goHex)
    }

    it("ProposerAddress") {
      val simpleBytes = Amino.encode(aminoHeader.proposerAddress, skipEmpty = false)
      val goSize = 21
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "1404C60B72246943675E2F3AADA00E30EC41AA7D4E"
      checkHex(scalaHex, goHex)
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
    val scalaHex = toHex(Amino.encode(arr, skipEmpty = false))
    val goHex = "0901020304060708090A"

    checkHex(scalaHex, goHex)
  }

  it("encode header") {
    val scalaHex = toHex(Amino.encodeLengthPrefixed(AminoBlock.toAminoHeader(block.header)))
    val goHex = ""

    checkHex(scalaHex, goHex)
  }

//  it("encode block") {
//    val scalaHex = toHex(Amino.encodeLengthPrefixed(AminoBlock.toAmino(block)))
//    println(s"scalaHex: $scalaHex")
//    val goHex =
//      "EB090AEB020A02080A120231301811220C08DBD4DCE50510F7E6E0FF012804303A3A480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B78542204B2FE08AE3C3F7573DB0A36250FB2AABCB1893638EBBECAB7356DDCEF358B4234A2061135E296D5570C607C456416CBD6827E21DED57F7FE2F08879ADCC8228FA7C05220E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F4753745A20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F4753746220048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F6A20DA265DD0F482C2D76B7E67B0128B391FED2971CD74E6DED3EDCCBAB200653DAE72207FD69E789D2A445E0F3258965481A2E70D5EB9960BCAF065B08C417EF045A84682011404C60B72246943675E2F3AADA00E30EC41AA7D4E1282020A46586553513152646F546B776F2F35340A435245415445205441424C4520757365727328696420696E742C206E616D65207661726368617228313238292C2061676520696E74290A46586553513152646F546B776F2F35360A435245415445205441424C4520757365727328696420696E742C206E616D65207661726368617228313238292C2061676520696E74290A37586553513152646F546B776F2F35350A494E5345525420494E544F2075736572732056414C55455328312C202753617261272C203233290A37586553513152646F546B776F2F35370A494E5345525420494E544F2075736572732056414C55455328312C202753617261272C2032332922F5040A480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B78512B4010802101022480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B7852A0C08DBD4DCE50510F7E6E0FF01321404C60B72246943675E2F3AADA00E30EC41AA7D4E4240674F7172B7F3F53EBEDEAD5893E831528DADA1D3C8EE679B29E77ACCEFA2D699C830F7456D2153F49263C68E49F472EB9F8C7616A7F741879F43C95018D33306120012B6010802101022480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B7852A0C08DBD4DCE50510F7E6E0FF013214991C9F03698AC07BEB41B71A87715FC4364A994A3802424056442271F8F16C6F84B078A6217AFCEDAEF0F0A9079C0ABF97AD02BFED2863EAED84B230EFB3693618EC3115D505388C27367769B4C1BC151BF63AF03C29520812B6010802101022480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B7852A0C08DBD4DCE50510F7E6E0FF0132149F16F63227F11942E6E4A3282B2A293E4BF8206C3803424037D3E5BA505F7D65DC5FFF884B3010DB70F568679727ECEF6100443EBBFEC451BB3AEBBBF09687093FB8E4CA7E43375861F8F5FBD5A13D3A5484855F93AEF600"
////    println(s"goHex: $goHex")
//
//    checkHex(scalaHex, goHex)
//  }
}
