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

package fluence.effects.tendermint.block.data

import fluence.crypto.hash.CryptoHashers.Sha256
import fluence.effects.tendermint.block.errors.TendermintBlockError
import fluence.effects.tendermint.block.protobuf.{Protobuf, ProtobufConverter, ProtobufJson}
import fluence.effects.tendermint.block.signature.Merkle
import io.circe.Decoder
import io.circe.generic.semiauto.deriveDecoder
import proto3.tendermint.Vote
import scodec.bits.ByteVector

object Block {
  /* JSON decoders */
  import Header._
  implicit final val decodeBase64ByteVector: Decoder[Base64ByteVector] = Decoder.decodeString.emap(
    str => ByteVector.fromBase64Descriptive(str).map(Base64ByteVector).left.map(_ => "Base64ByteVector")
  )
  implicit final val decodeVote: Decoder[Vote] =
    Decoder.decodeJson.emap(jvalue => ProtobufJson.vote(jvalue).left.map(_ => "Vote"))
  implicit final val dataDecoder: Decoder[Data] = deriveDecoder
  implicit final val lastCommitDecoder: Decoder[LastCommit] = deriveDecoder
  implicit final val blockDecoder: Decoder[Block] = deriveDecoder

  /* Definitions */
  type Parts = List[Array[Byte]]
  type Hash = Array[Byte]
  type Tx = Base64ByteVector
  type Evidence = Array[Byte]

  // Default block part size
  val BlockPartSizeBytes = 65536 // 64kB

  /**
   * Merkle hash of all precommits.
   *
   * Some of them could be None (null in JSON, nil in Go), such are serialized to [0x0, 0x0] (with prefix)
   * More details here: https://github.com/tendermint/go-amino/issues/260
   *
   * @param precommits List of possibly empty votes
   * @return Merkle hash of these votes
   */
  def commitHash(precommits: List[Option[Vote]]): Array[Byte] = {
    Merkle.simpleHash(precommits.map(Protobuf.encode(_)))
  }

  /**
   * Calculates merkle hash from the list of TXs
   */
  def txsHash(txs: List[Tx]): Array[Byte] = Merkle.simpleHash(txs.map(singleTxHash))

  /**
   * Calculates hash of the single tx
   * In Go code it is `tmhash.Sum(tx)` -> `SHA256.sum`
   */
  def singleTxHash(tx: Tx): Array[Byte] = Sha256.unsafe(tx.bv.toArray)

  /**
   * Calculates merkle hash of list of Evidence
   * TODO: Add Evidence to block, write tests on evidence
   */
  def evidenceHash(evl: List[Evidence]): Array[Byte] = Merkle.simpleHash(evl)

  def apply(json: String): Either[TendermintBlockError, Block] = {
    ProtobufJson.block(json)
  }
}

// TODO: Add Evidence field to the Block
case class Block private[block] (header: Header, data: Data, last_commit: LastCommit) {
  import Block._

  /**
   * MerkleRoot of the complete serialized block cut into parts (ie. MerkleRoot(MakeParts(block))
   * go: SimpleProofsFromByteSlices
   *
   * @return Parts header, containing hash and the number of parts
   */
  def partsHash(): PartsHeader = {
    val bytes = Protobuf.encodeLengthPrefixed(ProtobufConverter.toProtobuf(this))
    val parts = bytes.grouped(Block.BlockPartSizeBytes).toList
    val hash = Merkle.simpleHash(parts)
    PartsHeader(hash, parts.length)
  }

  /**
   * Calculates 3 hashes, should be called before blockHash()
   *
   * @return Copy of the Block with filled hashes
   */
  private def fillHeader(): Block = {
    val lastCommitHash = ByteVector(commitHash(last_commit.precommits))
    val dataHash = ByteVector(this.dataHash())
    val evHash = ByteVector(evidenceHash(Nil))

    copy(header.copy(last_commit_hash = lastCommitHash, data_hash = dataHash, evidence_hash = evHash))
  }

  /**
   * Calculates Merkle hash of the header
   *
   * @return Merkle hash of the header
   */
  def headerHash(): Array[Byte] = {
    fillHeader().filledHeaderHash()
  }

  /**
   * Calculates Merkle hash of the transaction list
   *
   * @return Merkle hash of the transaction list
   */
  def dataHash(): Array[Byte] = {
    data.txs.fold(Array.empty[Byte])(txsHash)
  }

  /**
   * Calculates Merkle hash of the lastCommit.precommits (votes for the previous block)
   *
   * @return Merkle hash of precommits
   */
  def lastCommitHash(): Array[Byte] = {
    commitHash(last_commit.precommits)
  }

  /**
   * Calculates Merkle hash from serialized header
   *
   * NOTE:
   *   In Tendermnt's Go code, header hash is calculated from `cdcEncode`-ed fields,
   *   which yields [] on empty arrays, that's why skipEmpty = true
   */
  private def filledHeaderHash(): Array[Byte] = {
    val data = List(
      Protobuf.encode(header.version),
      Protobuf.encode(header.chain_id),
      Protobuf.encode(header.height),
      Protobuf.encode(header.time),
      Protobuf.encode(header.num_txs),
      Protobuf.encode(header.total_txs),
      Protobuf.encode(header.last_block_id),
      Protobuf.encode(header.last_commit_hash, skipEmpty = true),
      Protobuf.encode(header.data_hash, skipEmpty = true),
      Protobuf.encode(header.validators_hash, skipEmpty = true),
      Protobuf.encode(header.next_validators_hash, skipEmpty = true),
      Protobuf.encode(header.consensus_hash, skipEmpty = true),
      Protobuf.encode(header.app_hash, skipEmpty = true),
      Protobuf.encode(header.last_results_hash, skipEmpty = true),
      Protobuf.encode(header.evidence_hash, skipEmpty = true),
      Protobuf.encode(header.proposer_address, skipEmpty = true)
    )

    Merkle.simpleHash(data)
  }
}
