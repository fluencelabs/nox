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

import fluence.effects.tendermint.block.data.Block
import fluence.effects.tendermint.block.protobuf.{Protobuf, ProtobufConverter, ProtobufJson}
import fluence.effects.tendermint.block.signature.Canonical
import io.circe.parser._
import org.scalatest.{FunSpec, Matchers, OptionValues}
import scodec.bits.ByteVector

class EncodingTest extends FunSpec with Matchers with OptionValues {

  val block = Block(TestData.blockResponse).right.get
  val vote = parse(TestData.vote).flatMap(ProtobufJson.vote).right.get

  def toHex(ba: Array[Byte]) = ByteVector(ba).toHex
  def checkHex(scalaHex: String, goHex: String) = scalaHex.toLowerCase shouldBe goHex.toLowerCase

  describe("header") {
    it("version") { checkHex(toHex(Protobuf.encode(block.header.version)), "080A") }

    it("chain_id") { checkHex(toHex(Protobuf.encode(block.header.chain_id)), "023130") }

    it("height") { checkHex(toHex(Protobuf.encode(block.header.height)), "11") }

    it("time") { checkHex(toHex(Protobuf.encode(block.header.time)), "08DBD4DCE50510F7E6E0FF01") }

    it("num_txs") { checkHex(toHex(Protobuf.encode(block.header.num_txs)), "04") }

    it("total_txs") { checkHex(toHex(Protobuf.encode(block.header.total_txs)), "3A") }

    it("last_block_id") {
      checkHex(
        toHex(Protobuf.encode(block.header.last_block_id)),
        "0A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      )
    }

    it("last_commit_hash") {
      checkHex(
        toHex(Protobuf.encode(block.header.last_commit_hash, skipEmpty = true)),
        "204B2FE08AE3C3F7573DB0A36250FB2AABCB1893638EBBECAB7356DDCEF358B423"
      )
    }

    it("data_hash") {
      checkHex(
        toHex(Protobuf.encode(block.header.data_hash, skipEmpty = true)),
        "2061135E296D5570C607C456416CBD6827E21DED57F7FE2F08879ADCC8228FA7C0"
      )
    }

    it("validators_hash") {
      checkHex(
        toHex(Protobuf.encode(block.header.validators_hash, skipEmpty = true)),
        "20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374"
      )
    }

    it("next_validators_hash") {
      checkHex(
        toHex(Protobuf.encode(block.header.next_validators_hash, skipEmpty = true)),
        "20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374"
      )
    }

    it("consensus_hash") {
      checkHex(
        toHex(Protobuf.encode(block.header.consensus_hash, skipEmpty = true)),
        "20048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F"
      )
    }

    it("app_hash") {
      checkHex(
        toHex(Protobuf.encode(block.header.app_hash, skipEmpty = true)),
        "20DA265DD0F482C2D76B7E67B0128B391FED2971CD74E6DED3EDCCBAB200653DAE"
      )
    }

    it("last_results_hash") {
      checkHex(
        toHex(Protobuf.encode(block.header.last_results_hash, skipEmpty = true)),
        "207FD69E789D2A445E0F3258965481A2E70D5EB9960BCAF065B08C417EF045A846"
      )
    }

    it("evidence_hash") {
      checkHex(toHex(Protobuf.encode(block.header.evidence_hash, skipEmpty = true)), "")
    }

    it("proposer_address") {
      checkHex(
        toHex(Protobuf.encode(block.header.proposer_address, skipEmpty = true)),
        "1404C60B72246943675E2F3AADA00E30EC41AA7D4E"
      )
    }
  }

  describe("amino header, length prefixed") {
    val aminoHeader = ProtobufConverter.toProtobuf(block.header)

    it("Version") {
      val simpleBytes = Protobuf.encode(aminoHeader.version)
      val goSize = 2
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "080A"
      checkHex(scalaHex, goHex)
    }

    it("ChainID") {
      val simpleBytes = Protobuf.encode(aminoHeader.chainId)
      val goSize = 3
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "023130"
      checkHex(scalaHex, goHex)
    }

    it("Height") {
      val simpleBytes = Protobuf.encode(aminoHeader.height)
      val goSize = 1
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "11"
      checkHex(scalaHex, goHex)
    }

    it("Time") {
      val simpleBytes = Protobuf.encode(aminoHeader.time)
      val goSize = 12
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "08DBD4DCE50510F7E6E0FF01"
      checkHex(scalaHex, goHex)
    }

    it("NumTxs") {
      val simpleBytes = Protobuf.encode(aminoHeader.numTxs)
      val goSize = 1
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "04"
      checkHex(scalaHex, goHex)
    }

    it("TotalTxs") {
      val simpleBytes = Protobuf.encode(aminoHeader.totalTxs)
      val goSize = 1
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "3A"
      checkHex(scalaHex, goHex)
    }

    it("LastBlockID") {
      val simpleBytes = Protobuf.encode(aminoHeader.lastBlockId)
      val goSize = 72
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex =
        "0A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      checkHex(scalaHex, goHex)
    }

    it("LastCommitHash") {
      val simpleBytes = Protobuf.encode(aminoHeader.lastCommitHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "204B2FE08AE3C3F7573DB0A36250FB2AABCB1893638EBBECAB7356DDCEF358B423"
      checkHex(scalaHex, goHex)
    }

    it("DataHash") {
      val simpleBytes = Protobuf.encode(aminoHeader.dataHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "2061135E296D5570C607C456416CBD6827E21DED57F7FE2F08879ADCC8228FA7C0"
      checkHex(scalaHex, goHex)
    }

    it("ValidatorsHash") {
      val simpleBytes = Protobuf.encode(aminoHeader.validatorsHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374"
      checkHex(scalaHex, goHex)
    }

    it("NextValidatorsHash") {
      val simpleBytes = Protobuf.encode(aminoHeader.nextValidatorsHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F475374"
      checkHex(scalaHex, goHex)
    }

    it("ConsensusHash") {
      val simpleBytes = Protobuf.encode(aminoHeader.consensusHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "20048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F"
      checkHex(scalaHex, goHex)
    }

    it("AppHash") {
      val simpleBytes = Protobuf.encode(aminoHeader.appHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "20DA265DD0F482C2D76B7E67B0128B391FED2971CD74E6DED3EDCCBAB200653DAE"
      checkHex(scalaHex, goHex)
    }

    it("LastResultsHash") {
      val simpleBytes = Protobuf.encode(aminoHeader.lastResultsHash, skipEmpty = false)
      val goSize = 33
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "207FD69E789D2A445E0F3258965481A2E70D5EB9960BCAF065B08C417EF045A846"
      checkHex(scalaHex, goHex)
    }

    it("EvidenceHash, skipEmpty = true") {
      val simpleBytes = Protobuf.encode(aminoHeader.evidenceHash, skipEmpty = true)
      val goSize = 0
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = ""
      checkHex(scalaHex, goHex)
    }

    it("EvidenceHash, skipEmpty = false") {
      val simpleBytes = Protobuf.encode(aminoHeader.evidenceHash, skipEmpty = false)
      val goSize = 1
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "00"
      checkHex(scalaHex, goHex)
    }

    it("ProposerAddress") {
      val simpleBytes = Protobuf.encode(aminoHeader.proposerAddress, skipEmpty = false)
      val goSize = 21
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = "1404C60B72246943675E2F3AADA00E30EC41AA7D4E"
      checkHex(scalaHex, goHex)
    }

    it("encode header") {
      val simpleBytes = Protobuf.encode(aminoHeader)
      val goSize = 363
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(Protobuf.encodeLengthPrefixed(aminoHeader))
      val goHex =
        "EB020A02080A120231301811220C08DBD4DCE50510F7E6E0FF012804303A3A480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B78542204B2FE08AE3C3F7573DB0A36250FB2AABCB1893638EBBECAB7356DDCEF358B4234A2061135E296D5570C607C456416CBD6827E21DED57F7FE2F08879ADCC8228FA7C05220E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F4753745A20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F4753746220048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F6A20DA265DD0F482C2D76B7E67B0128B391FED2971CD74E6DED3EDCCBAB200653DAE72207FD69E789D2A445E0F3258965481A2E70D5EB9960BCAF065B08C417EF045A84682011404C60B72246943675E2F3AADA00E30EC41AA7D4E"

      checkHex(scalaHex, goHex)
    }
  }

  it("encode vote") {
    val voteHex = toHex(Protobuf.encode(vote))
    val goHex =
      "0802101022480a201e56cf404964aa6b0768e67ad9cbacabcebcd6a84dc0fc924f1c0af9043c0188122408011220d0a00d1902638e1f4fd625568d4a4a7d9fc49e8f3586f257535fc835e7b0b7852a0c08dbd4dce50510f7e6e0ff01321404c60b72246943675e2f3aada00e30ec41aa7d4e4240674f7172b7f3f53ebedead5893e831528dada1d3c8ee679b29e77accefa2d699c830f7456d2153f49263c68e49f472eb9f8c7616a7f741879f43c95018d33306"
    checkHex(voteHex, goHex)
  }

  it("encode blockID") {
    block.header.last_block_id shouldBe defined
    val blockId = block.header.last_block_id.value
    val scalaHex = toHex(Protobuf.encode(blockId))
    val goHex =
      "0A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"

    checkHex(scalaHex, goHex)
  }

  it("encode Version") {
    block.header.version shouldBe defined
    val version = block.header.version.value
    val scalaHex = toHex(Protobuf.encode(version))
    val goHex = "080A"

    checkHex(scalaHex, goHex)
  }

  describe("amino block") {
    it("encode data") {
      val simpleBytes = Protobuf.encode(ProtobufConverter.toProtobuf(block.data))
      val goSize = 258
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex =
        "0A46586553513152646F546B776F2F35340A435245415445205441424C4520757365727328696420696E742C206E616D65207661726368617228313238292C2061676520696E74290A46586553513152646F546B776F2F35360A435245415445205441424C4520757365727328696420696E742C206E616D65207661726368617228313238292C2061676520696E74290A37586553513152646F546B776F2F35350A494E5345525420494E544F2075736572732056414C55455328312C202753617261272C203233290A37586553513152646F546B776F2F35370A494E5345525420494E544F2075736572732056414C55455328312C202753617261272C20323329"
      checkHex(scalaHex, goHex)
    }

    it("encode commit") {
      val simpleBytes = Protobuf.encode(ProtobufConverter.toProtobuf(block.last_commit))
      val goSize = 629

      val scalaHex = toHex(simpleBytes)
      goSize shouldBe simpleBytes.length
      val goHex =
        "0A480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B78512B4010802101022480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B7852A0C08DBD4DCE50510F7E6E0FF01321404C60B72246943675E2F3AADA00E30EC41AA7D4E4240674F7172B7F3F53EBEDEAD5893E831528DADA1D3C8EE679B29E77ACCEFA2D699C830F7456D2153F49263C68E49F472EB9F8C7616A7F741879F43C95018D33306120012B6010802101022480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B7852A0C08DBD4DCE50510F7E6E0FF013214991C9F03698AC07BEB41B71A87715FC4364A994A3802424056442271F8F16C6F84B078A6217AFCEDAEF0F0A9079C0ABF97AD02BFED2863EAED84B230EFB3693618EC3115D505388C27367769B4C1BC151BF63AF03C29520812B6010802101022480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B7852A0C08DBD4DCE50510F7E6E0FF0132149F16F63227F11942E6E4A3282B2A293E4BF8206C3803424037D3E5BA505F7D65DC5FFF884B3010DB70F568679727ECEF6100443EBBFEC451BB3AEBBBF09687093FB8E4CA7E43375861F8F5FBD5A13D3A5484855F93AEF600"
      checkHex(scalaHex, goHex)
    }

    it("encode block") {
      val scalaHex = toHex(Protobuf.encodeLengthPrefixed(ProtobufConverter.toProtobuf(block)))
      val goHex =
        "EB090AEB020A02080A120231301811220C08DBD4DCE50510F7E6E0FF012804303A3A480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B78542204B2FE08AE3C3F7573DB0A36250FB2AABCB1893638EBBECAB7356DDCEF358B4234A2061135E296D5570C607C456416CBD6827E21DED57F7FE2F08879ADCC8228FA7C05220E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F4753745A20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F4753746220048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F6A20DA265DD0F482C2D76B7E67B0128B391FED2971CD74E6DED3EDCCBAB200653DAE72207FD69E789D2A445E0F3258965481A2E70D5EB9960BCAF065B08C417EF045A84682011404C60B72246943675E2F3AADA00E30EC41AA7D4E1282020A46586553513152646F546B776F2F35340A435245415445205441424C4520757365727328696420696E742C206E616D65207661726368617228313238292C2061676520696E74290A46586553513152646F546B776F2F35360A435245415445205441424C4520757365727328696420696E742C206E616D65207661726368617228313238292C2061676520696E74290A37586553513152646F546B776F2F35350A494E5345525420494E544F2075736572732056414C55455328312C202753617261272C203233290A37586553513152646F546B776F2F35370A494E5345525420494E544F2075736572732056414C55455328312C202753617261272C2032332922F5040A480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B78512B4010802101022480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B7852A0C08DBD4DCE50510F7E6E0FF01321404C60B72246943675E2F3AADA00E30EC41AA7D4E4240674F7172B7F3F53EBEDEAD5893E831528DADA1D3C8EE679B29E77ACCEFA2D699C830F7456D2153F49263C68E49F472EB9F8C7616A7F741879F43C95018D33306120012B6010802101022480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B7852A0C08DBD4DCE50510F7E6E0FF013214991C9F03698AC07BEB41B71A87715FC4364A994A3802424056442271F8F16C6F84B078A6217AFCEDAEF0F0A9079C0ABF97AD02BFED2863EAED84B230EFB3693618EC3115D505388C27367769B4C1BC151BF63AF03C29520812B6010802101022480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188122408011220D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B7852A0C08DBD4DCE50510F7E6E0FF0132149F16F63227F11942E6E4A3282B2A293E4BF8206C3803424037D3E5BA505F7D65DC5FFF884B3010DB70F568679727ECEF6100443EBBFEC451BB3AEBBBF09687093FB8E4CA7E43375861F8F5FBD5A13D3A5484855F93AEF600"

      checkHex(scalaHex, goHex)
    }
  }

  describe("amino block with data = null") {
    val blockEmpty = Block(TestData.blockWithNullTxsResponse).right.get

    it("encode data") {
      val simpleBytes = Protobuf.encode(ProtobufConverter.toProtobuf(blockEmpty.data))
      val goSize = 0
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(simpleBytes)
      val goHex = ""
      checkHex(scalaHex, goHex)
    }

    it("encode block") {
      val simpleBytes = Protobuf.encode(ProtobufConverter.toProtobuf(blockEmpty))
      val goSize = 1140
      goSize shouldBe simpleBytes.length

      val scalaHex = toHex(Protobuf.encodeLengthPrefixed(ProtobufConverter.toProtobuf(blockEmpty)))
      val goHex =
        "F4080AC6020A02080A120231301813220B08F8DD81E60510FDEAD922303B3A480A20C5FDEC64C64E8EC018FD0090BAB9B3359C04B327D7E470674F357BD785025ACC1224080112205AE5A8746094CE0CDB3B4953908EFA4AB63330A999445C079B37A665E254818C42202F0EF75860A315E965412EAE57584EFC95877A91513A89ED4389B4DF3FFE256A5220E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F4753745A20E55E99B667008D3E275ECEEAF8807F1BFF36B30D6DD638CC57CC09939F4753746220048091BC7DDC283F77BFBF91D73C44DA58C3DF8A9CBC867405D8B7F3DAADA22F6A20CC0F66A511BBA4815752D83B253E9FD50F535A33C210F1121097F9C6FB58464172201A3CEF7A7D9A114F955678B2B37BCD6EF17712484EEE466F058E0E43BD66B90A820114991C9F03698AC07BEB41B71A87715FC4364A994A22A8060A480A20C5FDEC64C64E8EC018FD0090BAB9B3359C04B327D7E470674F357BD785025ACC1224080112205AE5A8746094CE0CDB3B4953908EFA4AB63330A999445C079B37A665E254818C12B3010802101222480A20C5FDEC64C64E8EC018FD0090BAB9B3359C04B327D7E470674F357BD785025ACC1224080112205AE5A8746094CE0CDB3B4953908EFA4AB63330A999445C079B37A665E254818C2A0B08F8DD81E60510E085D823321404C60B72246943675E2F3AADA00E30EC41AA7D4E4240F32DC93667061D1DF5943A903ACB925FDD8D367115199D2C25B6DE861F1122C0252173FDC128BD65C298B5655AE5E0F80093D8EEE7831B47D4ACEF09BBA6FB0512B5010802101222480A20C5FDEC64C64E8EC018FD0090BAB9B3359C04B327D7E470674F357BD785025ACC1224080112205AE5A8746094CE0CDB3B4953908EFA4AB63330A999445C079B37A665E254818C2A0B08F8DD81E60510FDEAD92232140D71FECA786E7FE982E6FC13422AAC82329DF0773801424026045986298DEEC318844B56A0F89D7C495EAA177BFC9C8166A456B7395DE1638A4D0D2C191E7C643DA82F802A9CF854E012B9628352ED9CB83D3B8BF719790012B5010802101222480A20C5FDEC64C64E8EC018FD0090BAB9B3359C04B327D7E470674F357BD785025ACC1224080112205AE5A8746094CE0CDB3B4953908EFA4AB63330A999445C079B37A665E254818C2A0B08F8DD81E60510AA94AD233214991C9F03698AC07BEB41B71A87715FC4364A994A38024240C4D4A3B51DFE6B52110B58CBCBFA39612F3FC7DB7D08919AAF8A0A4F2F21009A9A39BA0D5476B5FFE19BF3A56176403AC601D2240A1BBDB38E509E54DAB8920512B5010802101222480A20C5FDEC64C64E8EC018FD0090BAB9B3359C04B327D7E470674F357BD785025ACC1224080112205AE5A8746094CE0CDB3B4953908EFA4AB63330A999445C079B37A665E254818C2A0B08F8DD81E605109EAFD32232149F16F63227F11942E6E4A3282B2A293E4BF8206C38034240EA5F83425069CE0CEE10199FA3134AAADB1D13F13AB171903B7F56EDA5637CF114AE75EC7807CEF59365CCD5B5E022681EB2F977E0505609AE6939AAED6B3404"

      checkHex(scalaHex, goHex)
    }
  }

  describe("complex primitives") {
    it("encode string") {
      val s = "aaa"
      val scalaHex = toHex(Protobuf.encode(s))
      val goHex = "03616161"

      checkHex(scalaHex, goHex)
    }

    it("encode uint64") {
      val l = 123456789L
      val scalaHex = toHex(Protobuf.encode(l))
      val goHex = "959AEF3A"

      checkHex(scalaHex, goHex)
    }

    it("encode bytes") {
      val arr = ByteVector(1, 2, 3, 4, 6, 7, 8, 9, 10)
      val scalaHex = toHex(Protobuf.encode(arr, skipEmpty = false))
      val goHex = "0901020304060708090A"

      checkHex(scalaHex, goHex)
    }
  }

  describe("canonical") {
    val chainID = block.header.chain_id
    it("encode canonical vote") {

      val bytes = Protobuf.encodeLengthPrefixed(Canonical.vote(vote, chainID))
      val scalaHex = toHex(bytes)
      val goHex =
        "67080211100000000000000022480A201E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C018812240A20D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B78510012A0C08DBD4DCE50510F7E6E0FF0132023130"
      checkHex(scalaHex, goHex)
    }
  }
}
