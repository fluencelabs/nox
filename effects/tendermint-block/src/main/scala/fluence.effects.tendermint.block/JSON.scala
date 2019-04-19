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
import com.google.protobuf.timestamp.Timestamp
import io.circe.Decoder.Result
import io.circe.Json
import proto3.tendermint.{BlockID, Version, Vote}
import scalapb.lenses.{Lens, Mutation}
import scalapb_circe.Parser
import scodec.bits.ByteVector

object JSON {
  val parser = new Parser(true)

  val firstVote =
    """
      |{
      |    "type": 2,
      |    "height": 16,
      |    "round": 0,
      |    "block_id": {
      |        "hash": "1E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188",
      |        "parts": {
      |            "total": 1,
      |            "hash": "D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      |        }
      |    },
      |    "timestamp": "2019-04-17T13:30:03.536359799Z",
      |    "validator_address": "04C60B72246943675E2F3AADA00E30EC41AA7D4E",
      |    "validator_index": 0,
      |    "signature": "Z09xcrfz9T6+3q1Yk+gxUo2todPI7mebKed6zO+i1pnIMPdFbSFT9JJjxo5J9HLrn4x2Fqf3QYefQ8lQGNMzBg=="
      |}
    """.stripMargin

  val actualVote =
    """
      |{
      |  "type": 2,
      |  "height": "16",
      |  "round": "0",
      |  "block_id": {
      |    "hash": "1E56CF404964AA6B0768E67AD9CBACABCEBCD6A84DC0FC924F1C0AF9043C0188",
      |    "parts": {
      |      "total": "1",
      |      "hash": "D0A00D1902638E1F4FD625568D4A4A7D9FC49E8F3586F257535FC835E7B0B785"
      |    }
      |  },
      |  "timestamp": "2019-04-17T13:30:03.536359799Z",
      |  "validator_address": "991C9F03698AC07BEB41B71A87715FC4364A994A",
      |  "validator_index": "2",
      |  "signature": "VkQicfjxbG+EsHimIXr87a7w8KkHnAq/l60Cv+0oY+rthLIw77NpNhjsMRXVBTiMJzZ3abTBvBUb9jrwPClSCA=="
      |}
    """.stripMargin

  def block(blockResponse: String): Result[Block] = {
    import io.circe._, io.circe.parser._

    val p: Json = parse(blockResponse).right.get
    val block: Json = p.hcursor.downField("result").get[Json]("block").right.get
    block.as[Block]
  }

  def vote(json: Json): Vote = {
    val v = parser.fromJson[Vote](json)
    v.update(
      _.blockId.update(fixBlockId(): _*),
      _.validatorAddress.modify(fixBytes)
    )
  }

  def version(json: Json): Version = {
    parser.fromJson[Version](json)
  }

  def timestamp(json: Json): Timestamp = {
    parser.fromJson[Timestamp](json)
  }

  def blockId(json: Json): BlockID = {
    parser.fromJson[BlockID](json).update(fixBlockId(): _*)
  }

  private def fixBlockId(): List[Lens[BlockID, BlockID] => Mutation[BlockID]] = {
    val hash: Lens[BlockID, BlockID] => Mutation[BlockID] = _.hash.modify(fixBytes)
    val parts: Lens[BlockID, BlockID] => Mutation[BlockID] = _.parts.update(_.hash.modify(fixBytes))

    List(hash, parts)
  }

  /**
   * Performs base64 encode -> hex decode -> to byte string
   *
   * Protobuf "erroneously" applied `base64 decode` to a hex string, this function fixes that
   *
   * NOTE:
   *   It's not a protobuf mistake, it's just a protocol quirk.
   *   When Tendermint encodes values to JSON to return in RPC, some bytes (i.e., common.HexBytes) are encoded in hex,
   *   while other bytes (i.e., byte[]) are encoded in base64.
   *   It's just a happy coincidence that protobuf works on that JSON at all, it wasn't meant to.
   *
   * @param bs bs = ByteString.copyFrom(base64-decode(hexString))
   * @return Good, correct bytes, that were represented by a hexString
   */
  def fixBytes(bs: ByteString): ByteString = {
    val hex = ByteVector(bs.toByteArray).toBase64
    val value = ByteVector.fromHex(hex).getOrElse(throw new RuntimeException(s"Can't fromHex from $hex"))
    val array: Array[Byte] = value.toArray
    val bytes: ByteString = ByteString.copyFrom(array)
    bytes
  }
}
