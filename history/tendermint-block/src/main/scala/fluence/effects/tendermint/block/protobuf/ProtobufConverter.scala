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

import cats.Traverse
import cats.instances.either._
import cats.instances.list._
import com.google.protobuf.ByteString
import fluence.effects.tendermint.block.data._
import proto3.tendermint.Vote
import scodec.bits.ByteVector

import scala.language.higherKinds
import scala.util.control.NoStackTrace

/**
 * Collection of functions to convert Scala case classes to their protobuf counterparts
 * Main purpose is to serialize Block to protobuf bytes, and calculate Block.partsHash
 */
private[block] object ProtobufConverter {
  import proto3.tendermint.{Block => PBBlock, Commit => PBCommit, Data => PBData, Header => PBHeader}

  private def bs(bv: ByteVector): ByteString = ByteString.copyFrom(bv.toArray)
  private def bv(bs: ByteString): ByteVector = ByteVector(bs.asReadOnlyByteBuffer())

  /**
   * Encodes a list of optional protobuf Votes:
   * 1. To a default protobuf structure serialization, if element is defined
   * 2. To an empty byte array, if element is None
   *
   * Each empty byte array will become [0x1, 0x2, 0x0, 0x0] in PBCommit encoding
   * This is to be compatible with Tendermint's amino encoding, for details see https://github.com/tendermint/go-amino/issues/260
   */
  private def serialize(precommits: List[Option[Vote]]): List[ByteString] =
    Protobuf.encode(precommits).map(ByteString.copyFrom)

  def toProtobuf(lc: LastCommit) = PBCommit(Some(lc.block_id), serialize(lc.precommits))

  def toProtobuf(h: Header): PBHeader = {
    PBHeader(
      version = h.version,
      chainId = h.chain_id,
      height = h.height,
      time = h.time,
      numTxs = h.num_txs,
      totalTxs = h.total_txs,
      lastBlockId = h.last_block_id,
      lastCommitHash = bs(h.last_commit_hash),
      dataHash = bs(h.data_hash),
      validatorsHash = bs(h.validators_hash),
      nextValidatorsHash = bs(h.next_validators_hash),
      consensusHash = bs(h.consensus_hash),
      appHash = bs(h.app_hash),
      lastResultsHash = bs(h.last_results_hash),
      evidenceHash = bs(h.evidence_hash),
      proposerAddress = bs(h.proposer_address)
    )
  }

  def toProtobuf(d: Data): Option[PBData] = {
    d.txs.map(txs => PBData(txs.map(bv64 => bs(bv64.bv))))
  }

  def toProtobuf(b: Block): PBBlock = {
    val header = toProtobuf(b.header)
    val data = toProtobuf(b.data)

    PBBlock(
      header = Some(header),
      data = data,
      evidence = None,
      lastCommit = Some(toProtobuf(b.last_commit))
    )
  }

  def fromProtobuf(b: PBBlock): Either[Throwable, Block] =
    for {
      header <- getOr("header")(b.header).map(fromProtobuf)
      data = b.data.fold(Data(None))(fromProtobuf)
      lastCommit <- getOr("lastCommit")(b.lastCommit).flatMap(fromProtobuf)
    } yield Block(header, data, lastCommit)

  def fromProtobuf(d: PBData): Data = Data(Some(d.txs.map(bs => Base64ByteVector(bv(bs))).toList))

  def fromProtobuf(h: PBHeader): Header = {
    Header(
      version = h.version,
      chain_id = h.chainId,
      height = h.height,
      time = h.time,
      num_txs = h.numTxs,
      total_txs = h.totalTxs,
      last_block_id = h.lastBlockId,
      last_commit_hash = bv(h.lastCommitHash),
      data_hash = bv(h.dataHash),
      validators_hash = bv(h.validatorsHash),
      next_validators_hash = bv(h.nextValidatorsHash),
      consensus_hash = bv(h.consensusHash),
      app_hash = bv(h.appHash),
      last_results_hash = bv(h.lastResultsHash),
      evidence_hash = bv(h.evidenceHash),
      proposer_address = bv(h.proposerAddress),
    )
  }

  def fromProtobuf(b: PBCommit): Either[Throwable, LastCommit] =
    for {
      blockId <- getOr("blockId")(b.blockId)
      precommits <- Traverse[List].sequence(b.precommits.map(bs => Protobuf.decode[Vote](bs.toByteArray)).toList)
    } yield
      LastCommit(
        block_id = blockId,
        precommits = precommits.map(Some(_))
      )

  private def getOr[T](name: String)(opt: Option[T]): Either[Throwable, T] =
    Either.cond(opt.isDefined, opt.get, MissingField(name))
}

sealed trait ProtobufError extends NoStackTrace
case class MissingField(name: String) extends ProtobufError {
  override def toString: String = s"Missing field: $name"
}
