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

package fluence.effects.tendermint.block.protobuf

import com.google.protobuf.ByteString
import com.google.protobuf.timestamp.Timestamp
import fluence.effects.tendermint.block.data.{Block, Commit}
import fluence.effects.tendermint.block.errors.Errors._
import fluence.effects.tendermint.block.errors.TendermintBlockError
import io.circe.Json
import io.circe.parser._
import proto3.tendermint.{BlockID, Version, Vote}
import scalapb.lenses.{Lens, Mutation}
import scalapb_circe.Parser
import scodec.bits.ByteVector

import scala.language.postfixOps
import scala.util.Try

/**
 * Collection of functions that parse JSON strings to protobuf or scala classes
 */
private[block] object ProtobufJson {
  val parser = new Parser(true)

  /**
   * Parses block from Tendermint's RPC "Block" response
   *
   * @param blockResponse JSON string containing Tendermint's RPC response
   * @return Either an error or block
   */
  def block(blockResponse: String): Either[TendermintBlockError, Block] = {
    for {
      resposnseJson <- parse(blockResponse).convertError
      blockJson <- resposnseJson.hcursor.downField("result").get[Json]("block").convertError
      block <- blockJson.as[Block].convertError
    } yield block
  }

  /**
   * Parses commit from Tendermint's RPC "Commit" response
   *
   * @param commitResponse JSON string containing Tendermint's RPC response
   * @return Either an error or commit
   */
  def commit(commitResponse: String): Either[TendermintBlockError, Commit] = {
    for {
      responseJson <- parse(commitResponse).convertError
      commitJson <- responseJson.hcursor.downField("result").downField("signed_header").get[Json]("commit").convertError
      commit <- commitJson.as[Commit].convertError
    } yield commit
  }

  /**
   * Parses Json value into a protobuf Vote, re-encodes bytes where needed
   *
   * @param json Json value, parsed by circe
   * @return Either an error or protobuf Vote
   */
  def vote(json: Json): Either[ProtobufJsonError, Vote] = {
    Try(parser.fromJson[Vote](json)).toEither.convertError.map(
      _.update(
        _.blockId.update(fixBlockId(): _*),
        _.validatorAddress.modify(reencode)
      )
    )
  }

  /**
   * Parses Json value into a protobuf Version
   *
   * @param json Json value, parsed by circe
   * @return Either an error or protobuf Version
   */
  def version(json: Json): Either[ProtobufJsonError, Version] = {
    Try(parser.fromJson[Version](json)).toEither.convertError
  }

  /**
   * Parses Json value into a protobuf Timestamp
   *
   * @param json Json value, parsed by circe
   * @return Either an error or protobuf Timestamp
   */
  def timestamp(json: Json): Either[ProtobufJsonError, Timestamp] = {
    Try(parser.fromJson[Timestamp](json)).toEither.convertError
  }

  /**
   * Parses Json value into a protobuf BlockID
   *
   * @param json Json value, parsed by circe
   * @return Either an error or protobuf BlockID
   */
  def blockId(json: Json): Either[ProtobufJsonError, BlockID] = {
    Try(parser.fromJson[BlockID](json).update(fixBlockId(): _*)).toEither.convertError
  }

  /**
   * Re-encodes hashes inside BlockID, @see reencode doc for details
   *
   * @return List of lenses that reencode hashes in a BlockID
   */
  private def fixBlockId(): List[Lens[BlockID, BlockID] => Mutation[BlockID]] = {
    val hash: Lens[BlockID, BlockID] => Mutation[BlockID] = _.hash.modify(reencode)
    val parts: Lens[BlockID, BlockID] => Mutation[BlockID] = _.parts.update(_.hash.modify(reencode))

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
  private def reencode(bs: ByteString): ByteString = {
    val hex = ByteVector(bs.toByteArray).toBase64

    ByteVector
      .fromHexDescriptive(hex)
      .map { value =>
        val array: Array[Byte] = value.toArray
        val bytes: ByteString = ByteString.copyFrom(array)
        bytes
      }
      // Throwing an exception here, because `reencode` is used
      // with ScalaPB lenses, and they don't work with Either
      .fold(e => throw FixBytesError(e), identity) // TODO: don't throw exception, find a better way
  }
}
