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

import com.google.protobuf.ByteString
import fluence.effects.tendermint.block.data.{Block, PartsHeader}
import fluence.effects.tendermint.block.protobuf.ProtobufJson
import fluence.effects.tendermint.block.signature.TendermintSignature
import io.circe.parser.parse
import org.scalatest.{FunSpec, Matchers, OptionValues}
import scodec.bits.ByteVector

class TendermintSignatureTest extends FunSpec with Matchers with OptionValues {
  def toHex(ba: Array[Byte]): String = ByteVector(ba).toHex.toLowerCase
  def toHex(bs: ByteString): String = toHex(bs.toByteArray)

  it("verify vote") {
    val chainID = "10"
    val vote = parse(TestData.vote).flatMap(ProtobufJson.voteReencoded).right.get
    val validator = TestData.validators(vote.validatorIndex).toArray
    TendermintSignature.verifyVote(vote, chainID, validator) shouldBe true
  }

  it("verify commits") {
    val block = Block(TestData.blockResponse).right.get
    val commit = ProtobufJson.commit(TestData.commitResponse).right.get

    val chainId = block.header.chain_id
    val headerHash = toHex(block.headerHash())
    val PartsHeader(h, partsCount) = block.partsHash()
    val partsHash = toHex(h)

    commit.precommits.flatten.foreach { vote =>
      val id = vote.blockId.value
      toHex(id.hash) shouldBe headerHash

      val ps = id.parts.value
      ps.total shouldBe partsCount
      toHex(ps.hash) shouldBe partsHash

      TestData.validators.keySet should contain(vote.validatorIndex)
      val validator = TestData.validators(vote.validatorIndex).toArray
      TendermintSignature.verifyVote(vote, chainId, validator)
    }
  }
}
