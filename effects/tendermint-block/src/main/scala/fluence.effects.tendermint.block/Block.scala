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
import io.circe.Decoder
import io.circe.generic.semiauto.deriveDecoder
import proto3.tendermint.{BlockID, Commit, Vote}
import scodec.bits.ByteVector

import scala.util.Try

// About BlockID: https://tendermint.com/docs/spec/blockchain/blockchain.html#blockid

// newtype
case class Base64ByteVector(bv: ByteVector)
case class Data(txs: List[Base64ByteVector])

case class LastCommit(block_id: BlockID, precommits: List[Option[Vote]])

object Block {
  import Header._
  implicit final val decodeBase64ByteVector: Decoder[Base64ByteVector] = Decoder.decodeString.emap(
    str => ByteVector.fromBase64Descriptive(str).map(Base64ByteVector).left.map(_ => "Base64ByteVector")
  )
  implicit final val decodeVote: Decoder[Vote] =
    Decoder.decodeJson.emap(jvalue => Try(JSON.vote(jvalue)).toEither.left.map(_ => "Vote"))
  implicit final val dataDecoder: Decoder[Data] = deriveDecoder
  implicit final val lastCommitDecoder: Decoder[LastCommit] = deriveDecoder
  implicit final val blockDecoder: Decoder[Block] = deriveDecoder

  val BlockPartSizeBytes = 65536 // 64kB
}

// TODO: to/from JSON
// TODO: add evidence
case class Block(header: Header, data: Data, last_commit: LastCommit) {
  type Parts = List[Array[Byte]]
  type Hash = Array[Byte]
  type Tx = Base64ByteVector
  type Evidence = Array[Byte]
  type Precommits = List[Vote] // also, Vote = CommitSig in Go

  // SimpleHash, go: SimpleHashFromByteSlices
  // https://github.com/tendermint/tendermint/wiki/Merkle-Trees#simple-tree-with-dictionaries
  // MerkleRoot of all the fields in the header (ie. MerkleRoot(header))
  // Note:
  //    We will abuse notion and invoke MerkleRoot with arguments of type struct or type []struct.
  //    For struct arguments, we compute a [][]byte containing the amino encoding of each field in the
  //    struct, in the same order the fields appear in the struct. For []struct arguments, we compute a
  //    [][]byte by amino encoding the individual struct elements.
  def blockHash(): Hash = headerHash()

  // used for secure gossipping of the block during consensus
  def makeParts(): Parts = {
    val bytes = Amino.encodeLengthPrefixed(this)
    ???
  }
  // MerkleRoot of the complete serialized block cut into parts (ie. MerkleRoot(MakeParts(block))
  // go: SimpleProofsFromByteSlices
  def partsHash(): Hash = ???

  // Calculates 3 hashes, should be called before blockHash()
  def fillHeader(): Block = {
    val lastCommitHash = ByteVector(commitHash(last_commit.precommits))
    val dataHash = ByteVector(txsHash(data.txs))
    val evHash = ByteVector(evidenceHash(Nil))

    copy(header.copy(last_commit_hash = lastCommitHash, data_hash = dataHash, evidence_hash = evHash))
  }

  def headerHash(): Array[Byte] = {
    fillHeader().filledHeaderHash()
  }

  private def filledHeaderHash(): Array[Byte] = {
    val data = List(
      Amino.encode(header.version),
      Amino.encode(header.chain_id),
      Amino.encode(header.height),
      Amino.encode(header.time),
      Amino.encode(header.num_txs),
      Amino.encode(header.total_txs),
      Amino.encode(header.last_block_id),
      Amino.encode(header.last_commit_hash),
      Amino.encode(header.data_hash),
      Amino.encode(header.validators_hash),
      Amino.encode(header.next_validators_hash),
      Amino.encode(header.consensus_hash),
      Amino.encode(header.app_hash),
      Amino.encode(header.last_results_hash),
      Amino.encode(header.evidence_hash),
      Amino.encode(header.proposer_address),
    )

    Merkle.simpleHash(data)
  }

  // Merkle hash of all precommits (some of them could be null?)
  def commitHash(precommits: List[Option[Vote]]) = {
    Merkle.simpleHash(precommits.map(Amino.encode(_)))
  }

  // Merkle hash from the list of TXs
  def txsHash(txs: List[Tx]) = Merkle.simpleHash(txs.map(singleTxHash))

  // Hash of the single tx, go: tmhash.Sum(tx) -> SHA256.sum
  def singleTxHash(tx: Tx) = SHA256.sum(tx.bv.toArray)

  def evidenceHash(evl: List[Evidence]) = Merkle.simpleHash(evl)
}

object AminoBlock {
  import proto3.tendermint.{Block => PBBlock}

  def toAmino(b: Block): PBBlock = {
    def bs(bv: ByteVector): ByteString = ByteString.copyFrom(bv.toArray)
    def toCommit(lc: LastCommit) = Commit(Some(lc.block_id), lc.precommits.flatten)

    PBBlock(
      version =            b.header.version,
      chainId =            b.header.chain_id,
      height =             b.header.height,
      time =               b.header.time,
      numTxs =             b.header.num_txs,
      totalTxs =           b.header.total_txs,
      lastBlockId =        b.header.last_block_id,
      lastCommitHash =     bs(b.header.last_commit_hash),
      dataHash =           bs(b.header.data_hash),
      validatorsHash =     bs(b.header.validators_hash),
      nextValidatorsHash = bs(b.header.next_validators_hash),
      consensusHash =      bs(b.header.consensus_hash),
      appHash =            bs(b.header.app_hash),
      lastResultsHash =    bs(b.header.last_results_hash),
      evidenceHash =       bs(b.header.evidence_hash),
      proposerAddress =    bs(b.header.proposer_address),
      txs =                b.data.txs.map(bv64 => bs(bv64.bv)),
      evidence =           None,
      lastCommit =         Some(toCommit(b.last_commit)),
    )
  }
}

object SHA256 {
  import fluence.crypto.hash.CryptoHashers.Sha256

  def sum(bs: Array[Byte]): Array[Byte] = Sha256.unsafe(bs)
}
